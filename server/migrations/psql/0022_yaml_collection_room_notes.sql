ALTER TABLE collection_room_store ADD COLUMN notes TEXT DEFAULT '' NOT NULL;

DROP VIEW collection_room;

CREATE VIEW collection_room WITH (security_barrier='false', security_invoker='true') AS
    SELECT r.*, u.discord_username AS owner_discord_username
    FROM collection_room_store r
    INNER JOIN ct_user u ON u.id = r.owner_ct_user_id;

CREATE RULE collection_room_delete_store AS
    ON DELETE TO collection_room
    DO INSTEAD
    DELETE FROM collection_room_store
        WHERE collection_room_store.id = old.id
    RETURNING
        collection_room_store.*,
        (
            SELECT discord_username
            FROM ct_user
            WHERE ct_user.id = collection_room_store.owner_ct_user_id
        ) AS owner_discord_username;

CREATE RULE collection_room_insert_store AS
    ON INSERT TO collection_room
    DO INSTEAD
    INSERT INTO collection_room_store (id, owner_ct_user_id, title, created_at, closes_at, is_closed, notes)
    VALUES (
        new.id,
        new.owner_ct_user_id,
        new.title,
        new.created_at,
        new.closes_at,
        new.is_closed,
        new.notes
    )
    RETURNING
        collection_room_store.*,
        (
            SELECT discord_username
            FROM ct_user
            WHERE ct_user.id = collection_room_store.owner_ct_user_id
        ) AS owner_discord_username;

CREATE RULE collection_room_update_store AS
    ON UPDATE TO collection_room
    DO INSTEAD
    UPDATE collection_room_store
    SET
        id = new.id,
        owner_ct_user_id = new.owner_ct_user_id,
        title = new.title,
        created_at = new.created_at,
        closes_at = new.closes_at,
        is_closed = new.is_closed,
        notes = new.notes
    WHERE collection_room_store.id = old.id
    RETURNING
        collection_room_store.*,
        (
            SELECT discord_username
            FROM ct_user
            WHERE ct_user.id = collection_room_store.owner_ct_user_id
        ) AS owner_discord_username;
