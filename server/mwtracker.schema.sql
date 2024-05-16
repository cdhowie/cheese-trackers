--
-- PostgreSQL database dump
--

-- Dumped from database version 16.1 (Debian 16.1-1.pgdg120+1)
-- Dumped by pg_dump version 16.1

-- Started on 2024-02-04 07:54:52 UTC

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- TOC entry 845 (class 1247 OID 16386)
-- Name: game_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.game_status AS ENUM (
    'unblocked',
    'bk',
    'all_checks',
    'done',
    'open',
    'released',
    'glitched'
);


--
-- TOC entry 857 (class 1247 OID 16483)
-- Name: tracker_game_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.tracker_game_status AS ENUM (
    'disconnected',
    'connected',
    'goal_completed',
    'playing'
);


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- TOC entry 215 (class 1259 OID 16401)
-- Name: ap_game; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ap_game (
    id integer NOT NULL,
    tracker_id integer NOT NULL,
    name text NOT NULL,
    game text NOT NULL,
    checks_done integer NOT NULL,
    checks_total integer NOT NULL,
    last_activity timestamp with time zone,
    discord_username text,
    discord_ping boolean DEFAULT false NOT NULL,
    status public.game_status DEFAULT 'unblocked'::public.game_status NOT NULL,
    last_checked timestamp with time zone,
    "position" integer NOT NULL,
    tracker_status public.tracker_game_status NOT NULL
);


--
-- TOC entry 216 (class 1259 OID 16408)
-- Name: ap_hint; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ap_hint (
    id integer NOT NULL,
    finder_game_id integer NOT NULL,
    receiver_game_id integer NOT NULL,
    item text NOT NULL,
    location text NOT NULL,
    entrance text NOT NULL,
    found boolean NOT NULL
);


--
-- TOC entry 217 (class 1259 OID 16413)
-- Name: ap_hint_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.ap_hint_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- TOC entry 3388 (class 0 OID 0)
-- Dependencies: 217
-- Name: ap_hint_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ap_hint_id_seq OWNED BY public.ap_hint.id;


--
-- TOC entry 218 (class 1259 OID 16414)
-- Name: ap_tracker; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ap_tracker (
    id integer NOT NULL,
    tracker_id text NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


--
-- TOC entry 220 (class 1259 OID 16420)
-- Name: ap_tracker_game_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.ap_tracker_game_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- TOC entry 3389 (class 0 OID 0)
-- Dependencies: 220
-- Name: ap_tracker_game_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ap_tracker_game_id_seq OWNED BY public.ap_game.id;


--
-- TOC entry 219 (class 1259 OID 16419)
-- Name: ap_tracker_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.ap_tracker_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- TOC entry 3390 (class 0 OID 0)
-- Dependencies: 219
-- Name: ap_tracker_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ap_tracker_id_seq OWNED BY public.ap_tracker.id;


--
-- TOC entry 3219 (class 2604 OID 16421)
-- Name: ap_game id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game ALTER COLUMN id SET DEFAULT nextval('public.ap_tracker_game_id_seq'::regclass);


--
-- TOC entry 3222 (class 2604 OID 16422)
-- Name: ap_hint id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint ALTER COLUMN id SET DEFAULT nextval('public.ap_hint_id_seq'::regclass);


--
-- TOC entry 3223 (class 2604 OID 16423)
-- Name: ap_tracker id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_tracker ALTER COLUMN id SET DEFAULT nextval('public.ap_tracker_id_seq'::regclass);


--
-- TOC entry 3232 (class 2606 OID 16425)
-- Name: ap_hint ap_hint_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint
    ADD CONSTRAINT ap_hint_pkey PRIMARY KEY (id);


--
-- TOC entry 3225 (class 2606 OID 16431)
-- Name: ap_game ap_tracker_game_name_idx; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game
    ADD CONSTRAINT ap_tracker_game_name_idx UNIQUE (tracker_id, name);


--
-- TOC entry 3227 (class 2606 OID 16433)
-- Name: ap_game ap_tracker_game_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game
    ADD CONSTRAINT ap_tracker_game_pkey PRIMARY KEY (id);


--
-- TOC entry 3229 (class 2606 OID 16435)
-- Name: ap_game ap_tracker_game_position_idx; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game
    ADD CONSTRAINT ap_tracker_game_position_idx UNIQUE (tracker_id, "position");


--
-- TOC entry 3234 (class 2606 OID 16427)
-- Name: ap_tracker ap_tracker_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_tracker
    ADD CONSTRAINT ap_tracker_pkey PRIMARY KEY (id);


--
-- TOC entry 3236 (class 2606 OID 16429)
-- Name: ap_tracker ap_tracker_tracker_id_idx; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_tracker
    ADD CONSTRAINT ap_tracker_tracker_id_idx UNIQUE (tracker_id);


--
-- TOC entry 3230 (class 1259 OID 16436)
-- Name: fki_ap_tracker_game_tracker_id_fkey; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fki_ap_tracker_game_tracker_id_fkey ON public.ap_game USING btree (tracker_id);


--
-- TOC entry 3238 (class 2606 OID 16437)
-- Name: ap_hint ap_hint_finder_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint
    ADD CONSTRAINT ap_hint_finder_game_id_fkey FOREIGN KEY (finder_game_id) REFERENCES public.ap_game(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- TOC entry 3239 (class 2606 OID 16442)
-- Name: ap_hint ap_hint_receiver_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint
    ADD CONSTRAINT ap_hint_receiver_game_id_fkey FOREIGN KEY (receiver_game_id) REFERENCES public.ap_game(id);


--
-- TOC entry 3237 (class 2606 OID 16447)
-- Name: ap_game ap_tracker_game_tracker_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game
    ADD CONSTRAINT ap_tracker_game_tracker_id_fkey FOREIGN KEY (tracker_id) REFERENCES public.ap_tracker(id) ON UPDATE CASCADE ON DELETE CASCADE;


-- Completed on 2024-02-04 07:54:52 UTC

--
-- PostgreSQL database dump complete
--

ALTER TABLE ONLY public.ap_game
    ADD COLUMN notes text NOT NULL DEFAULT '';

ALTER TABLE public.ap_tracker
    ADD COLUMN title text DEFAULT '';

CREATE TYPE public.ping_preference AS ENUM
    ('liberally', 'sparingly', 'hints', 'see_notes', 'never');

ALTER TABLE public.ap_game
	ALTER COLUMN discord_ping DROP DEFAULT,
	ALTER COLUMN discord_ping TYPE public.ping_preference USING 'never'::public.ping_preference,
	ALTER COLUMN discord_ping SET DEFAULT 'never'::public.ping_preference;

ALTER TABLE public.ap_hint
    ALTER COLUMN receiver_game_id DROP NOT NULL;

ALTER TYPE public.game_status
    ADD VALUE 'unknown' AFTER 'glitched';
ALTER TYPE public.game_status
    ADD VALUE 'goal' AFTER 'unknown';

CREATE TABLE public.ct_user
(
    id serial NOT NULL,
    discord_access_token text NOT NULL,
    discord_access_token_expires_at timestamp with time zone NOT NULL,
    discord_refresh_token text NOT NULL,
    discord_username text NOT NULL,
    discord_user_id bigint NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT ct_user_discord_user_id_idx UNIQUE (discord_user_id)
);

ALTER TABLE public.ap_game
    ADD COLUMN claimed_by_ct_user_id integer;

ALTER TABLE public.ap_game
    ADD CONSTRAINT ap_game_claimed_by_ct_user_id_fkey FOREIGN KEY (claimed_by_ct_user_id)
    REFERENCES public.ct_user (id) MATCH SIMPLE
    ON UPDATE CASCADE
    ON DELETE SET NULL;

CREATE INDEX fki_ap_game_claimed_by_ct_user_id_fkey
    ON public.ap_game(claimed_by_ct_user_id);

CREATE TABLE public.js_error
(
    id serial NOT NULL,
    ct_user_id integer,
    error text NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT js_error_ct_user_id_fkey FOREIGN KEY (ct_user_id)
        REFERENCES public.ct_user (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE SET NULL
);

CREATE TYPE public.progression_status AS ENUM
    ('unknown', 'unblocked', 'bk');

CREATE TYPE public.completion_status AS ENUM
    ('incomplete', 'all_checks', 'goal', 'done', 'released');

CREATE TYPE public.availability_status AS ENUM
    ('unknown', 'open', 'claimed', 'public');

ALTER TABLE public.ap_game
    ADD COLUMN availability_status availability_status NOT NULL DEFAULT 'unknown'::availability_status;

UPDATE ap_game SET availability_status = CASE
	WHEN status = 'open' THEN CASE
		WHEN discord_username IS NOT NULL THEN 'public'
		ELSE 'open'
	END
	ELSE CASE
		WHEN discord_username IS NOT NULL THEN 'claimed'
		ELSE 'unknown'
	END
END::availability_status;

ALTER TABLE public.ap_game
    ADD COLUMN completion_status completion_status NOT NULL DEFAULT 'incomplete'::completion_status;

UPDATE ap_game SET completion_status = CASE
	WHEN status IN ('released', 'glitched') THEN 'released'
	WHEN status IN ('unknown', 'unblocked', 'bk', 'open') THEN 'incomplete'
	ELSE status::text
END::completion_status;

ALTER TABLE public.ap_game
    ADD COLUMN progression_status progression_status NOT NULL DEFAULT 'unknown'::progression_status;

UPDATE ap_game SET progression_status = CASE
	WHEN status IN ('all_checks', 'goal', 'open') THEN 'unknown'
	WHEN status IN ('done', 'released', 'glitched') THEN 'unblocked'
	ELSE status::text
END::progression_status;

ALTER TABLE public.ap_game DROP COLUMN status;

ALTER TABLE public.ap_tracker
    ADD COLUMN owner_ct_user_id integer;

ALTER TABLE public.ap_tracker
    ADD CONSTRAINT ap_tracker_owner_ct_user_id_fkey FOREIGN KEY (owner_ct_user_id)
    REFERENCES public.ct_user (id) MATCH SIMPLE
    ON UPDATE CASCADE
    ON DELETE SET NULL;

CREATE INDEX fki_ap_tracker_owner_ct_user_id_fkey
    ON public.ap_tracker(owner_ct_user_id);

ALTER TYPE public.progression_status
    ADD VALUE 'go' AFTER 'bk';

ALTER TABLE public.ap_tracker
    ADD COLUMN lock_title boolean NOT NULL DEFAULT FALSE;

ALTER TYPE public.tracker_game_status
    ADD VALUE 'ready' AFTER 'playing';

BEGIN;

    ALTER TABLE public.ap_game
        RENAME TO ap_game_store;

    -- This constraint should have used the CASCADE action already.
    ALTER TABLE public.ap_hint
        DROP CONSTRAINT ap_hint_receiver_game_id_fkey,
        ADD CONSTRAINT ap_hint_receiver_game_id_fkey FOREIGN KEY (receiver_game_id)
            REFERENCES public.ap_game_store (id) MATCH SIMPLE
            ON UPDATE CASCADE
            ON DELETE CASCADE;

    CREATE VIEW public.ap_game
        WITH (security_invoker=true)
    AS
        SELECT
            g.*,
            COALESCE(u.discord_username, g.discord_username) AS effective_discord_username
        FROM ap_game_store g
        LEFT JOIN ct_user u ON u.id = g.claimed_by_ct_user_id;

    CREATE RULE ap_game_insert_store AS
        ON INSERT TO public.ap_game
        DO INSTEAD
    (
        INSERT INTO ap_game_store (
            id,
            tracker_id,
            name,
            game,
            checks_done,
            checks_total,
            last_activity,
            discord_username,
            last_checked,
            position,
            tracker_status,
            notes,
            discord_ping,
            claimed_by_ct_user_id,
            availability_status,
            completion_status,
            progression_status
        )
        VALUES (
            NEW.id,
            NEW.tracker_id,
            NEW.name,
            NEW.game,
            NEW.checks_done,
            NEW.checks_total,
            NEW.last_activity,
            NEW.discord_username,
            NEW.last_checked,
            NEW.position,
            NEW.tracker_status,
            NEW.notes,
            NEW.discord_ping,
            NEW.claimed_by_ct_user_id,
            NEW.availability_status,
            NEW.completion_status,
            NEW.progression_status
        )
        RETURNING
            *,
            COALESCE(
                (SELECT discord_username FROM ct_user u WHERE u.id = claimed_by_ct_user_id),
                discord_username
            ) AS effective_discord_username
    );

    CREATE RULE ap_game_update_store AS
        ON UPDATE TO public.ap_game
        DO INSTEAD
    (
        UPDATE ap_game_store SET
            id = NEW.id,
            tracker_id = NEW.tracker_id,
            name = NEW.name,
            game = NEW.game,
            checks_done = NEW.checks_done,
            checks_total = NEW.checks_total,
            last_activity = NEW.last_activity,
            discord_username = NEW.discord_username,
            last_checked = NEW.last_checked,
            position = NEW.position,
            tracker_status = NEW.tracker_status,
            notes = NEW.notes,
            discord_ping = NEW.discord_ping,
            claimed_by_ct_user_id = NEW.claimed_by_ct_user_id,
            availability_status = NEW.availability_status,
            completion_status = NEW.completion_status,
            progression_status = NEW.progression_status
        WHERE id = OLD.id
        RETURNING
            *,
            COALESCE(
                (SELECT u.discord_username FROM ct_user u WHERE u.id = claimed_by_ct_user_id),
                discord_username
            ) AS effective_discord_username
    );

    CREATE RULE ap_game_delete_store AS
        ON DELETE TO public.ap_game
        DO INSTEAD
    (
        DELETE FROM ap_game_store
        WHERE id = OLD.id
        RETURNING
            *,
            COALESCE(
                (SELECT discord_username FROM ct_user u WHERE u.id = claimed_by_ct_user_id),
                discord_username
            ) AS effective_discord_username
    );

COMMIT;

ALTER VIEW ap_game ALTER COLUMN id SET DEFAULT nextval('ap_tracker_game_id_seq');

CREATE FUNCTION public.get_dashboard_trackers(IN uid integer)
    RETURNS TABLE (
        id integer,
        tracker_id text,
        title text,
        owner_ct_user_id integer,
        owner_discord_username text,
        last_activity timestamp with time zone
    )
    LANGUAGE 'sql'
    STABLE 
AS $BODY$
    WITH ut (id) AS (
        SELECT id
        FROM ap_tracker t
        WHERE owner_ct_user_id = uid

        UNION

        SELECT tracker_id 
        FROM ap_game_store
        WHERE claimed_by_ct_user_id = uid
    )

    SELECT
        t.id,
        t.tracker_id,
        t.title,
        t.owner_ct_user_id,
        u.discord_username AS owner_discord_username,
        gs.last_activity

    FROM ap_tracker t
    LEFT OUTER JOIN ct_user u
        ON u.id = t.owner_ct_user_id

    INNER JOIN (
        SELECT
            tracker_id,
            MAX(last_activity) AS last_activity,
            MIN(
                CASE completion_status
                    WHEN 'done' THEN 1
                    WHEN 'goal' THEN 1
                    WHEN 'released' THEN 1
                    ELSE 0
                END
            ) = 1 AS all_done

        FROM ap_game_store
        WHERE tracker_id IN (SELECT id FROM ut)
        GROUP BY tracker_id
    ) gs
        ON gs.tracker_id = t.id

    WHERE t.id IN (SELECT id FROM ut)
        AND NOT gs.all_done
$BODY$;

BEGIN;

    CREATE TYPE public.hint_classification AS ENUM
        ('unknown', 'critical', 'useful', 'trash');

    ALTER TABLE IF EXISTS public.ap_hint
        ADD COLUMN classification hint_classification NOT NULL DEFAULT 'unknown';

COMMIT;
