use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use escape_string::escape;
use musicbrainz_rs::Search;
use once_cell::sync::Lazy;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

use crate::database::{self, RiaArtistType, RiaGender};
use crate::entities::{prelude::*, *};
use crate::media::{store_artist_directory, store_audio_artist};

static MUSICBRAINZ_LAST_REQUEST: Lazy<Arc<RwLock<u64>>> = Lazy::new(|| Arc::new(RwLock::new(0)));

// According to https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting the MusicBrainz API
// allows ~50 requests per second. We make a maximum of 1 request every 2 seconds, which is only
// 30 requests per second, well below this upper limit. It would take ~33 minutes to process
// 1,000 artists.
const MINIMUM_DELAY: u64 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueuePayload {
    pub(crate) payload_type: PayloadType,
    pub(crate) id: i32,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum PayloadType {
    AudioArtist,
}

#[instrument]
pub(crate) async fn process_queue() {
    event!(Level::TRACE, "process_queue");

    // Ensure that we query the Musicbrainz API no more than once every 2 seconds.
    loop {
        let time_since_last_request = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - *MUSICBRAINZ_LAST_REQUEST.read().unwrap();

        if time_since_last_request >= MINIMUM_DELAY {
            event!(Level::TRACE, "process next queue item");
            if let Some(item) = load_from_queue().await {
                // @TODO: Error handling.
                let payload: QueuePayload = serde_json::from_str(&item.payload.unwrap()).unwrap();
                match payload.payload_type {
                    PayloadType::AudioArtist => {
                        if let Some(artist_id) = load_artist_by_name(&payload.value).await {
                            store_audio_artist(payload.id, artist_id).await;
                            store_artist_directory(payload.id, artist_id).await;
                            remove_from_queue(item.musicbrainz_queue_id).await;
                        }
                    }
                }
            } else {
                event!(Level::TRACE, "the queue is empty");
                tokio::time::sleep(tokio::time::Duration::from_secs(MINIMUM_DELAY)).await;
            }
        } else {
            // Only sleep if the musicbrainz api was invoked.
            let sleep_for = MINIMUM_DELAY - time_since_last_request;
            event!(Level::TRACE, "sleep {} seconds", sleep_for);
            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_for)).await;
        }
    }
}

#[instrument]
pub(crate) async fn add_to_queue(payload: QueuePayload) {
    event!(Level::TRACE, "add_to_queue");

    // Convert payload to JSON String. @TODO: error handling.
    let payload_json = serde_json::to_string(&payload).unwrap();

    // Be sure the payload isn't already in the queue.
    let queue_id = {
        let db = database::connection().await;
        match MusicbrainzQueue::find()
            .filter(musicbrainz_queue::Column::Payload.like(&payload_json))
            .one(db)
            .await
        {
            Ok(e) => e,
            Err(e) => {
                event!(
                    Level::WARN,
                    "MusicbrainzQueue::find() failure in add_to_queue: {}",
                    e
                );
                return;
            }
        }
    };

    // Only add payload if not already in the queue.
    if queue_id.is_none() {
        let queue_item = musicbrainz_queue::ActiveModel {
            created_at: Set(chrono::Utc::now().naive_utc().to_owned()),
            payload: Set(Some(payload_json.to_owned())),
            ..Default::default()
        };
        let db = database::connection().await;
        if let Err(error) = queue_item.insert(db).await {
            event!(
                Level::WARN,
                "failed to insert {} into musicbrainz_queue: {}",
                payload_json,
                error
            )
        }
    } else {
        event!(
            Level::TRACE,
            "'{}' payload is already in queue, not adding again",
            payload_json
        );
    }
}

#[instrument]
pub(crate) async fn load_from_queue() -> Option<musicbrainz_queue::Model> {
    event!(Level::TRACE, "load_from_queue");

    let queue_item = {
        let db = database::connection().await;
        // @TODO: wrap in a lock, or turn this into a subquery as described at
        // https://blabosoft.com/implementing-queue-in-postgresql. Lock is probably
        // better for keeping this database-agnostic.
        match MusicbrainzQueue::find()
            .filter(musicbrainz_queue::Column::ProcessingStartedAt.is_null())
            .order_by_asc(musicbrainz_queue::Column::CreatedAt)
            .one(db)
            .await
        {
            Ok(e) => e,
            Err(e) => {
                event!(Level::WARN, "MusicbrainzQueue::find() failure: {}", e);
                return None;
            }
        }
    };
    if let Some(item) = queue_item {
        let queue_id = item.musicbrainz_queue_id;
        event!(Level::TRACE, "next queue_id: {}", queue_id);
        let mut item: musicbrainz_queue::ActiveModel = item.into();
        item.processing_started_at = Set(Some(chrono::Utc::now().naive_utc().to_owned()));
        let db = database::connection().await;
        match item.update(db).await {
            Ok(i) => Some(i),
            Err(e) => {
                event!(
                    Level::WARN,
                    "failed to load id {} from musicbrainz_queue: {}",
                    queue_id,
                    e
                );
                None
            }
        }
    } else {
        None
    }
}

#[instrument]
pub(crate) async fn remove_from_queue(id: i32) {
    event!(Level::TRACE, "remove_from_queue");

    let db = database::connection().await;
    if let Err(e) = MusicbrainzQueue::delete_by_id(id).exec(db).await {
        event!(
            Level::WARN,
            "failed to delete id {} from musicbrainz_queue: {}",
            id,
            e
        );
    };
}

#[instrument]
pub(crate) async fn load_artist_by_name(artist_name: &str) -> Option<i32> {
    let artist_name_escaped = escape(artist_name);
    event!(
        Level::ERROR,
        "load_artist_by_name escaped: {}",
        artist_name_escaped
    );

    // Check if the artist is already in the database.
    let existing = {
        let db = database::connection().await;
        match Artist::find()
            .filter(artist::Column::Name.like(&artist_name_escaped))
            .one(db)
            .await
        {
            Ok(e) => e,
            Err(e) => {
                event!(Level::WARN, "Artist::find() failure: {}", e);
                return None;
            }
        }
    };

    event!(Level::ERROR, "{:#?}", existing);

    if let Some(artist) = existing {
        event!(Level::TRACE, "artist exists in database: {:#?}", artist);
        return Some(artist.artist_id);
    }

    let query = musicbrainz_rs::entity::artist::Artist::query_builder()
        .name(&artist_name_escaped)
        .build();

    // Update global tracking last request to MusicBrainz API to allow throttling requests.
    {
        let mut last_request = MUSICBRAINZ_LAST_REQUEST.write().unwrap();
        *last_request = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    let query_result = match musicbrainz_rs::entity::artist::Artist::search(query).execute() {
        Ok(q) => q,
        Err(e) => {
            event!(Level::WARN, "musicbrainz query failed: {}", e);
            return None;
        }
    };

    let artist = if let Some(result) =
        // For now assume the first matching artist.
        query_result.entities.get(0)
    {
        event!(Level::INFO, "MusicBrainz response: {:#?}", result);

        let mut area_id = 0;

        // If area is defined, track it in the database.
        if let Some(area) = &result.area {
            let existing_area = {
                let db = database::connection().await;
                match ArtistArea::find()
                    .filter(artist_area::Column::Name.like(&area.name))
                    .one(db)
                    .await
                {
                    Ok(e) => e,
                    Err(e) => {
                        event!(Level::WARN, "ArtistArea::find() failure: {}", e);
                        return None;
                    }
                }
            };
            area_id = if let Some(area) = existing_area {
                area.artist_area_id
            } else {
                let new_area = artist_area::ActiveModel {
                    // @TODO:
                    area_type: ActiveValue::Set("".to_string()),
                    name: ActiveValue::Set(area.name.to_string()),
                    sort_name: ActiveValue::Set(area.sort_name.to_string()),
                    disambiguation: ActiveValue::Set(area.disambiguation.to_string()),
                    ..Default::default()
                };
                event!(Level::DEBUG, "Insert ArtistArea: {:?}", new_area);
                let new_artist_area = {
                    let db = database::connection().await;
                    ArtistArea::insert(new_area)
                        .exec(db)
                        .await
                        .expect("failed to write artist to database")
                };
                new_artist_area.last_insert_id
            };
        }

        // Artist AreaId is optional.
        let artist_area_id = if area_id > 0 { Some(area_id) } else { None };

        // ArtistType is optional, convert to RiaArtistType to add
        // SeaOrm mapping.
        let artist_type: Option<RiaArtistType> = result
            .artist_type
            .as_ref()
            .map(|a| a.try_into().expect("ArtistType conversion can't fail"));

        // Gender is optional, convert to RiaGender to add
        // SeaOrm mapping.
        let gender: Option<RiaGender> = result
            .gender
            .as_ref()
            .map(|g| g.try_into().expect("Gender conversion can't fail"));

        artist::ActiveModel {
            name: ActiveValue::Set(artist_name.to_string()),
            musicbrainz_name: ActiveValue::Set(result.name.to_string()),
            musicbrainz_id: ActiveValue::Set(result.id.to_string()),
            sort_name: ActiveValue::Set(result.sort_name.to_string()),
            disambiguation_comment: ActiveValue::Set(result.disambiguation.to_string()),
            artist_area_id: ActiveValue::Set(artist_area_id),
            artist_type: ActiveValue::Set(artist_type),
            gender: ActiveValue::Set(gender),
            ..Default::default()
        }
    } else {
        event!(
            Level::WARN,
            "{} not found in MusicBrainz",
            artist_name_escaped
        );
        artist::ActiveModel {
            name: ActiveValue::Set(artist_name_escaped.to_string()),
            ..Default::default()
        }
    };

    event!(Level::DEBUG, "Insert Artist: {:?}", artist);
    let new_artist = {
        let db = database::connection().await;
        Artist::insert(artist)
            .exec(db)
            .await
            .expect("failed to write artist to database")
    };
    Some(new_artist.last_insert_id)
}
