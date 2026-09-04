CREATE TABLE collection_room_store (
    id uuid NOT NULL PRIMARY KEY,
    owner_ct_user_id INTEGER NOT NULL REFERENCES ct_user (id) ON DELETE CASCADE ON UPDATE CASCADE,
    title TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    closes_at TIMESTAMP WITH TIME ZONE NULL,
    is_closed BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_collection_room_owner_ct_user_id_lookup ON collection_room_store (owner_ct_user_id);

CREATE TABLE collection_room_slot_store (
    id SERIAL NOT NULL PRIMARY KEY,
    collection_room_id uuid NOT NULL REFERENCES collection_room_store (id) ON DELETE CASCADE ON UPDATE CASCADE,
    owner_ct_user_id INTEGER NOT NULL REFERENCES ct_user (id) ON DELETE CASCADE ON UPDATE CASCADE,
    changed_at TIMESTAMP WITH TIME ZONE NOT NULL,
    slot_name TEXT NOT NULL CHECK (
        length(slot_name) > 0 AND
        length(regexp_replace(slot_name, '{(player|PLAYER|number|NUMBER)}', '0000', 'g')) <= 16
    ),
    slot_game TEXT NOT NULL CHECK (length(slot_game) > 0),
    notes TEXT NOT NULL,
    yaml TEXT NOT NULL
);

CREATE INDEX idx_collection_room_slot_owner_ct_user_id_lookup ON collection_room_slot_store (owner_ct_user_id);

CREATE INDEX idx_collection_room_slot_collection_room_id_lookup ON collection_room_slot_store (collection_room_id);

CREATE UNIQUE INDEX idx_collection_room_slot_slot_name_unique
    ON collection_room_slot_store (collection_room_id, slot_name)
    WHERE slot_name NOT LIKE '%{player}%'
        AND slot_name NOT LIKE '%{PLAYER}%'
        AND slot_name NOT LIKE '%{number}%'
        AND slot_name NOT LIKE '%{NUMBER}%';

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
    INSERT INTO collection_room_store (id, owner_ct_user_id, title, created_at, closes_at, is_closed)
    VALUES (
        new.id,
        new.owner_ct_user_id,
        new.title,
        new.created_at,
        new.closes_at,
        new.is_closed
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
        is_closed = new.is_closed
    WHERE collection_room_store.id = old.id
    RETURNING
        collection_room_store.*,
        (
            SELECT discord_username
            FROM ct_user
            WHERE ct_user.id = collection_room_store.owner_ct_user_id
        ) AS owner_discord_username;

CREATE VIEW collection_room_slot WITH (security_barrier='false', security_invoker='true') AS
    SELECT s.*, u.discord_username AS owner_discord_username
    FROM collection_room_slot_store s
    INNER JOIN ct_user u ON u.id = s.owner_ct_user_id;

ALTER TABLE collection_room_slot ALTER COLUMN id SET DEFAULT nextval('public.collection_room_slot_store_id_seq'::regclass);

CREATE RULE collection_room_slot_delete_store AS
    ON DELETE TO collection_room_slot
    DO INSTEAD
    DELETE FROM collection_room_slot_store
        WHERE collection_room_slot_store.id = old.id
    RETURNING
        collection_room_slot_store.*,
        (
            SELECT discord_username
            FROM ct_user
            WHERE ct_user.id = collection_room_slot_store.owner_ct_user_id
        ) AS owner_discord_username;

CREATE RULE collection_room_slot_insert_store AS
    ON INSERT TO collection_room_slot
    DO INSTEAD
    INSERT INTO collection_room_slot_store (id, collection_room_id, owner_ct_user_id, changed_at, slot_name, slot_game, notes, yaml)
    VALUES (
        new.id,
        new.collection_room_id,
        new.owner_ct_user_id,
        new.changed_at,
        new.slot_name,
        new.slot_game,
        new.notes,
        new.yaml
    )
    RETURNING
        collection_room_slot_store.*,
        (
            SELECT discord_username
            FROM ct_user
            WHERE ct_user.id = collection_room_slot_store.owner_ct_user_id
        ) AS owner_discord_username;

CREATE RULE collection_room_slot_update_store AS
    ON UPDATE TO collection_room_slot
    DO INSTEAD
    UPDATE collection_room_slot_store
    SET
        id = new.id,
        collection_room_id = new.collection_room_id,
        owner_ct_user_id = new.owner_ct_user_id,
        changed_at = new.changed_at,
        slot_name = new.slot_name,
        slot_game = new.slot_game,
        notes = new.notes,
        yaml = new.yaml
    WHERE collection_room_slot_store.id = old.id
    RETURNING
        collection_room_slot_store.*,
        (
            SELECT discord_username
            FROM ct_user
            WHERE ct_user.id = collection_room_slot_store.owner_ct_user_id
        ) AS owner_discord_username;
