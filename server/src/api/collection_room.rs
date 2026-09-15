use std::{
    io::{Cursor, Read},
    sync::Arc,
};

use axum::{
    Json,
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequest, Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use futures::{TryFutureExt, TryStreamExt};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{IgnoredAny, MapAccess, Visitor},
};
use uuid::Uuid;
use yaml_split::YamlSplitError;
use zip::{ZipArchive, result::ZipError};

use crate::{
    ap_api::UrlEncodedUuid,
    auth::token::AuthenticatedUser,
    db::{
        DataAccess, DataAccessProvider, Transactable, Transaction,
        model::{
            CollectionRoom, CollectionRoomIden, CollectionRoomInsertion,
            CollectionRoomSlotInsertion,
        },
    },
    logging::UnexpectedResultExt,
    response::ErrIntoResponse,
    state::AppState,
};

#[derive(Serialize)]
pub struct ApiCollectionRoom {
    pub id: UrlEncodedUuid,
    pub owner_ct_user_id: i32,
    pub owner_discord_username: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub closes_at: Option<DateTime<Utc>>,
    pub is_closed: bool,
    pub notes: String,
}

impl From<CollectionRoom> for ApiCollectionRoom {
    fn from(value: CollectionRoom) -> Self {
        ApiCollectionRoom {
            id: value.id.into(),
            owner_ct_user_id: value.owner_ct_user_id,
            owner_discord_username: value.owner_discord_username,
            title: value.title,
            created_at: value.created_at,
            closes_at: value.closes_at,
            is_closed: value.is_closed,
            notes: value.notes,
        }
    }
}

/// `GET /collection_room/{room_id}`: Get collection room.
pub async fn get_collection_room<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(room_id): Path<UrlEncodedUuid>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let room: ApiCollectionRoom = db
        .get_collection_room_by_id(room_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?
        .into();

    Ok(Json(room))
}

#[derive(Debug, Deserialize)]
pub struct CollectionRoomRequestBody {
    pub title: String,
    pub closes_at: Option<DateTime<Utc>>,
    pub is_closed: bool,
    pub notes: String,
}

/// `POST /collection_room`: Create a new collection room.
pub async fn create_collection_room<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
    Json(room): Json<CollectionRoomRequestBody>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let room = CollectionRoomInsertion {
        id: Uuid::new_v4(),
        owner_ct_user_id: user.user.id,
        title: room.title,
        created_at: Utc::now(),
        closes_at: room.closes_at,
        is_closed: room.is_closed,
        notes: room.notes,
    };

    tokio::pin! {
        let new_rooms = db.create_collection_rooms([room]);
    };

    let room: ApiCollectionRoom = new_rooms
        .try_next()
        .await
        .unexpected()?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .into();

    Ok(Json(room))
}

/// `PUT /collection_room/{room_id}`: Update collection room.
pub async fn update_collection_room<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
    Path(room_id): Path<UrlEncodedUuid>,
    Json(room): Json<CollectionRoomRequestBody>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let mut existing = tx
        .get_collection_room_by_id(room_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    if existing.owner_ct_user_id != user.user.id {
        return Err(StatusCode::FORBIDDEN);
    }

    existing.title = room.title;
    existing.closes_at = room.closes_at;
    existing.is_closed = room.is_closed;
    existing.notes = room.notes;

    let updated: ApiCollectionRoom = tx
        .update_collection_room(
            existing,
            &[
                CollectionRoomIden::Title,
                CollectionRoomIden::ClosesAt,
                CollectionRoomIden::IsClosed,
                CollectionRoomIden::Notes,
            ],
        )
        .await
        .unexpected()?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .into();

    tx.commit().await.unexpected()?;

    Ok(Json(updated))
}

/// `GET /user/self/collection_room`: Get user's own collection rooms.
pub async fn get_own_collection_rooms<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let rooms: Vec<_> = db
        .get_collection_rooms_by_owner(user.user.id)
        .map_ok(ApiCollectionRoom::from)
        .try_collect()
        .await
        .unexpected()?;

    Ok(Json(rooms))
}

/// `GET: /collection_room/{room_id}/slot`: Get slots.
pub async fn get_collection_room_slots<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(room_id): Path<UrlEncodedUuid>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    // We don't need the room, just make sure it exists.
    tx.get_collection_room_by_id(room_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    let slots: Vec<_> = tx
        .get_collection_room_slots_by_collection_room_id(room_id.into())
        .try_collect()
        .await
        .unexpected()?;

    Ok(Json(slots))
}

pub struct LimitedBytes<const BYTE_LIMIT: usize>(Bytes);

impl<const BYTE_LIMIT: usize, S> FromRequest<S> for LimitedBytes<BYTE_LIMIT>
where
    S: Send + Sync,
{
    type Rejection = <Bytes as FromRequest<S>>::Rejection;

    fn from_request(
        mut req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        DefaultBodyLimit::max(BYTE_LIMIT).apply(&mut req);

        Bytes::from_request(req, state).map_ok(Self)
    }
}

#[derive(thiserror::Error, Debug)]
enum YamlUploadError {
    #[error("Not a ZIP archive: {0}")]
    InvalidZip(ZipError),
    #[error("ZIP archive error: {0}")]
    Zip(#[from] ZipError),
    #[error("ZIP archive contains too many files")]
    ZipTooManyEntries,
    #[error("ZIP entry size overflow")]
    ZipEntrySizeOverflow,
    #[error("Could not split YAML: {0}")]
    YamlSplit(#[from] YamlSplitError),
    #[error("Could not parse YAML: {0}")]
    YamlParse(#[from] yaml_serde::Error),
    #[error("YAML is missing game object")]
    MissingGameObject,
}

impl IntoResponse for YamlUploadError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
    }
}

fn zip_to_slot_yamls(bytes: &[u8]) -> Result<Vec<Bytes>, YamlUploadError> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(YamlUploadError::InvalidZip)?;

    if zip.len() > 1000 {
        return Err(YamlUploadError::ZipTooManyEntries);
    }

    (0..zip.len())
        .filter_map(|i| {
            (|| {
                let mut file = zip.by_index(i)?;

                if !file.is_file() {
                    return Ok(None);
                }

                // 5MB limit on individual YAMLs.
                let size = match file.size().try_into() {
                    Ok(v) if v <= 5 * 1024 * 1024 => v,
                    _ => return Err(YamlUploadError::ZipEntrySizeOverflow),
                };

                let mut contents = vec![0u8; size];

                file.read_exact(&mut contents[..]).map_err(ZipError::from)?;

                Ok(Some(contents.into()))
            })()
            .transpose()
        })
        .collect()
}

#[derive(Debug)]
struct UploadedSlot {
    name: String,
    game: String,
    yaml: String,
}

fn yaml_has_game_object(yaml: &str, game: &str) -> bool {
    struct IgnoredMap;

    impl<'de> Visitor<'de> for IgnoredMap {
        type Value = Self;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            while let Some((IgnoredAny, IgnoredAny)) = map.next_entry()? {}

            Ok(Self)
        }
    }

    impl<'de> Deserialize<'de> for IgnoredMap {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_map(IgnoredMap)
        }
    }

    struct HasGameObjectVisitor<'a> {
        game: &'a str,
    }

    impl<'de> Visitor<'de> for HasGameObjectVisitor<'_> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "a map with a single key \"{}\", containing a map",
                self.game,
            )
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut has_game = false;

            while let Some(key) = map.next_key::<String>()? {
                if key == self.game {
                    let IgnoredMap = map.next_value()?;
                    has_game = true;
                } else {
                    let IgnoredAny = map.next_value()?;
                }
            }

            match has_game {
                true => Ok(()),
                false => Err(serde::de::Error::custom(format!(
                    "document does not have a map at key {:?}",
                    self.game
                ))),
            }
        }
    }

    yaml_serde::Deserializer::from_str(yaml)
        .deserialize_map(HasGameObjectVisitor { game })
        .is_ok()
}

fn remove_prefix(s: &mut String, prefix: &str) -> bool {
    if s.starts_with(prefix) {
        s.drain(..prefix.len());
        true
    } else {
        false
    }
}

fn uploaded_bytes_to_slot_yamls(bytes: Bytes) -> Result<Vec<UploadedSlot>, YamlUploadError> {
    let yaml_files = match zip_to_slot_yamls(bytes.as_ref()) {
        Ok(slots) => slots,

        Err(YamlUploadError::InvalidZip(_)) => {
            // If not a text file, assume it's a single UTF-8 encoded YAML file.
            vec![bytes]
        }

        Err(e) => return Err(e),
    };

    #[derive(Deserialize)]
    struct ArchipelagoSlot {
        name: String,
        game: String,
    }

    yaml_files
        .into_iter()
        .flat_map(|yaml| {
            yaml_split::DocumentIterator::new(Cursor::new(yaml)).map(|doc| {
                let mut doc = doc?;

                // Normalize newlines.
                doc = doc.replace("\r\n", "\n").replace("\r", "\n");

                remove_prefix(&mut doc, "\u{FEFF}");

                // Strip document separator from the beginning, if present.
                // Otherwise, look for a separator somewhere in the document.
                // What comes before it would be a YAML header (such as a %YAML
                // directive).  Discard this completely; we don't support
                // headers, and they will make concatenating files problematic.
                if !remove_prefix(&mut doc, "---\n")
                    && let Some(pos) = doc.find("\n---\n")
                {
                    doc.drain(..(pos + 5));
                }

                // Replace all trailing whitespace with a single newline.
                {
                    let trimmed_len = doc.trim_end().len();
                    doc.drain(trimmed_len..);
                    doc.push('\n');
                }

                let slot: ArchipelagoSlot = yaml_serde::from_str(&doc)?;

                match yaml_has_game_object(&doc, &slot.game) {
                    true => Ok(UploadedSlot {
                        name: slot.name,
                        game: slot.game,
                        yaml: doc,
                    }),

                    false => Err(YamlUploadError::MissingGameObject),
                }
            })
        })
        .collect()
}

/// `POST: /collection_room/{room_id}/slot`: Create slots.
pub async fn create_collection_room_slots<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
    Path(room_id): Path<UrlEncodedUuid>,
    // 5MB upload limit
    LimitedBytes(body): LimitedBytes<{ 5 * 1024 * 1024 }>,
) -> Result<impl IntoResponse, Response>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let slots = tokio::task::spawn_blocking(move || uploaded_bytes_to_slot_yamls(body))
        .await
        .unwrap()
        .err_into_response()?;

    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()
        .err_into_response()?;

    let mut tx = db.begin().await.unexpected().err_into_response()?;

    let room = tx
        .get_collection_room_by_id(room_id.into())
        .await
        .unexpected()
        .err_into_response()?
        .ok_or(StatusCode::NOT_FOUND)
        .err_into_response()?;

    if room.is_effectively_closed() {
        return Err(StatusCode::FORBIDDEN.into_response());
    }

    // Try inserting the rooms.  If we get a unique violation error, we will
    // have to sort out which slot caused the conflict.
    //
    // To do that, we need to preserve the names of the slots we are trying to
    // create.
    let new_slot_names: Vec<_> = slots.iter().map(|i| i.name.clone()).collect();

    let changed_at = Utc::now();

    let created_slots: Result<Vec<_>, _> = tx
        .create_collection_room_slots(slots.into_iter().map(|s| CollectionRoomSlotInsertion {
            collection_room_id: room.id,
            owner_ct_user_id: user.user.id,
            changed_at,
            slot_name: s.name,
            slot_game: s.game,
            notes: String::new(),
            yaml: s.yaml,
        }))
        .try_collect()
        .await;

    let created_slots = match created_slots {
        // Likely a duplicate slot name.
        Err(e)
            if e.as_database_error()
                .is_some_and(|e| e.is_unique_violation()) =>
        {
            // TODO: Figure out which slot.
            return Err(StatusCode::UNPROCESSABLE_ENTITY.into_response());
        }

        // Likely a slot name that is too long.
        Err(e)
            if e.as_database_error()
                .is_some_and(|e| e.is_check_violation()) =>
        {
            // TODO: Figure out which slot.
            return Err(StatusCode::UNPROCESSABLE_ENTITY.into_response());
        }

        v => v,
    }
    .unexpected()
    .err_into_response()?;

    tx.commit().await.unexpected().err_into_response()?;

    Ok(Json(created_slots))
}

/// `DELETE: /collection_room/{room_id}/slot/{slot_id}`: Delete slot.
pub async fn delete_collection_room_slot<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
    Path((room_id, slot_id)): Path<(UrlEncodedUuid, i32)>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let room = tx
        .get_collection_room_by_id(room_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    let slot = tx
        .get_collection_room_slot_by_id(slot_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Room owner can always delete slots.  Slot owner can only delete slots if
    // the room is not closed.
    let can_delete = room.owner_ct_user_id == user.user.id
        || (!room.is_effectively_closed() && slot.owner_ct_user_id == user.user.id);

    if !can_delete {
        return Err(StatusCode::FORBIDDEN);
    }

    tx.delete_collection_room_slot(slot_id).await.unexpected()?;

    tx.commit().await.unexpected()?;

    Ok(Json(slot))
}
