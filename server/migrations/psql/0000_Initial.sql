--
-- PostgreSQL database dump
--

-- Dumped from database version 16.1 (Debian 16.1-1.pgdg120+1)
-- Dumped by pg_dump version 16.1 (Debian 16.1-1.pgdg120+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: availability_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.availability_status AS ENUM (
    'unknown',
    'open',
    'claimed',
    'public'
);


--
-- Name: completion_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.completion_status AS ENUM (
    'incomplete',
    'all_checks',
    'goal',
    'done',
    'released'
);


--
-- Name: game_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.game_status AS ENUM (
    'unblocked',
    'bk',
    'all_checks',
    'done',
    'open',
    'released',
    'glitched',
    'unknown',
    'goal'
);


--
-- Name: hint_classification; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.hint_classification AS ENUM (
    'unknown',
    'critical',
    'useful',
    'trash'
);


--
-- Name: ping_preference; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.ping_preference AS ENUM (
    'liberally',
    'sparingly',
    'hints',
    'see_notes',
    'never'
);


--
-- Name: progression_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.progression_status AS ENUM (
    'unknown',
    'unblocked',
    'bk',
    'go',
    'soft_bk'
);


--
-- Name: tracker_game_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.tracker_game_status AS ENUM (
    'disconnected',
    'connected',
    'goal_completed',
    'playing',
    'ready'
);


--
-- Name: get_dashboard_trackers(integer); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.get_dashboard_trackers(uid integer) RETURNS TABLE(id integer, tracker_id text, title text, owner_ct_user_id integer, owner_discord_username text, last_activity timestamp with time zone)
    LANGUAGE sql STABLE
    AS $$
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
$$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: ap_game_store; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ap_game_store (
    id integer NOT NULL,
    tracker_id integer NOT NULL,
    name text NOT NULL,
    game text NOT NULL,
    checks_done integer NOT NULL,
    checks_total integer NOT NULL,
    last_activity timestamp with time zone,
    discord_username text,
    last_checked timestamp with time zone,
    "position" integer NOT NULL,
    tracker_status public.tracker_game_status NOT NULL,
    notes text DEFAULT ''::text NOT NULL,
    discord_ping public.ping_preference DEFAULT 'never'::public.ping_preference NOT NULL,
    claimed_by_ct_user_id integer,
    availability_status public.availability_status DEFAULT 'unknown'::public.availability_status NOT NULL,
    completion_status public.completion_status DEFAULT 'incomplete'::public.completion_status NOT NULL,
    progression_status public.progression_status DEFAULT 'unknown'::public.progression_status NOT NULL
);


--
-- Name: ct_user; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ct_user (
    id integer NOT NULL,
    discord_access_token text NOT NULL,
    discord_access_token_expires_at timestamp with time zone NOT NULL,
    discord_refresh_token text NOT NULL,
    discord_username text NOT NULL,
    discord_user_id bigint NOT NULL
);


--
-- Name: ap_game; Type: VIEW; Schema: public; Owner: -
--

CREATE VIEW public.ap_game WITH (security_barrier='false', security_invoker='true') AS
 SELECT g.id,
    g.tracker_id,
    g.name,
    g.game,
    g.checks_done,
    g.checks_total,
    g.last_activity,
    g.discord_username,
    g.last_checked,
    g."position",
    g.tracker_status,
    g.notes,
    g.discord_ping,
    g.claimed_by_ct_user_id,
    g.availability_status,
    g.completion_status,
    g.progression_status,
    COALESCE(u.discord_username, g.discord_username) AS effective_discord_username
   FROM (public.ap_game_store g
     LEFT JOIN public.ct_user u ON ((u.id = g.claimed_by_ct_user_id)));


--
-- Name: ap_hint; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ap_hint (
    id integer NOT NULL,
    finder_game_id integer NOT NULL,
    receiver_game_id integer,
    item text NOT NULL,
    location text NOT NULL,
    entrance text NOT NULL,
    found boolean NOT NULL,
    classification public.hint_classification DEFAULT 'unknown'::public.hint_classification NOT NULL
);


--
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
-- Name: ap_hint_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ap_hint_id_seq OWNED BY public.ap_hint.id;


--
-- Name: ap_tracker; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.ap_tracker (
    id integer NOT NULL,
    tracker_id text NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    title text DEFAULT ''::text,
    owner_ct_user_id integer,
    lock_title boolean DEFAULT false NOT NULL
);


--
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
-- Name: ap_tracker_game_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ap_tracker_game_id_seq OWNED BY public.ap_game_store.id;


--
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
-- Name: ap_tracker_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ap_tracker_id_seq OWNED BY public.ap_tracker.id;


--
-- Name: ct_user_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.ct_user_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: ct_user_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.ct_user_id_seq OWNED BY public.ct_user.id;


--
-- Name: js_error; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.js_error (
    id integer NOT NULL,
    ct_user_id integer,
    error text NOT NULL
);


--
-- Name: js_error_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.js_error_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: js_error_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.js_error_id_seq OWNED BY public.js_error.id;


--
-- Name: ap_game id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game ALTER COLUMN id SET DEFAULT nextval('public.ap_tracker_game_id_seq'::regclass);


--
-- Name: ap_game_store id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game_store ALTER COLUMN id SET DEFAULT nextval('public.ap_tracker_game_id_seq'::regclass);


--
-- Name: ap_hint id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint ALTER COLUMN id SET DEFAULT nextval('public.ap_hint_id_seq'::regclass);


--
-- Name: ap_tracker id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_tracker ALTER COLUMN id SET DEFAULT nextval('public.ap_tracker_id_seq'::regclass);


--
-- Name: ct_user id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ct_user ALTER COLUMN id SET DEFAULT nextval('public.ct_user_id_seq'::regclass);


--
-- Name: js_error id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.js_error ALTER COLUMN id SET DEFAULT nextval('public.js_error_id_seq'::regclass);


--
-- Name: ap_hint ap_hint_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint
    ADD CONSTRAINT ap_hint_pkey PRIMARY KEY (id);


--
-- Name: ap_game_store ap_tracker_game_name_idx; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game_store
    ADD CONSTRAINT ap_tracker_game_name_idx UNIQUE (tracker_id, name);


--
-- Name: ap_game_store ap_tracker_game_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game_store
    ADD CONSTRAINT ap_tracker_game_pkey PRIMARY KEY (id);


--
-- Name: ap_game_store ap_tracker_game_position_idx; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game_store
    ADD CONSTRAINT ap_tracker_game_position_idx UNIQUE (tracker_id, "position");


--
-- Name: ap_tracker ap_tracker_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_tracker
    ADD CONSTRAINT ap_tracker_pkey PRIMARY KEY (id);


--
-- Name: ap_tracker ap_tracker_tracker_id_idx; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_tracker
    ADD CONSTRAINT ap_tracker_tracker_id_idx UNIQUE (tracker_id);


--
-- Name: ct_user ct_user_discord_user_id_idx; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ct_user
    ADD CONSTRAINT ct_user_discord_user_id_idx UNIQUE (discord_user_id);


--
-- Name: ct_user ct_user_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ct_user
    ADD CONSTRAINT ct_user_pkey PRIMARY KEY (id);


--
-- Name: js_error js_error_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.js_error
    ADD CONSTRAINT js_error_pkey PRIMARY KEY (id);


--
-- Name: fki_ap_game_claimed_by_ct_user_id_fkey; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fki_ap_game_claimed_by_ct_user_id_fkey ON public.ap_game_store USING btree (claimed_by_ct_user_id);


--
-- Name: fki_ap_tracker_game_tracker_id_fkey; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fki_ap_tracker_game_tracker_id_fkey ON public.ap_game_store USING btree (tracker_id);


--
-- Name: fki_ap_tracker_owner_ct_user_id_fkey; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX fki_ap_tracker_owner_ct_user_id_fkey ON public.ap_tracker USING btree (owner_ct_user_id);


--
-- Name: ap_game ap_game_delete_store; Type: RULE; Schema: public; Owner: -
--

CREATE RULE ap_game_delete_store AS
    ON DELETE TO public.ap_game DO INSTEAD  DELETE FROM public.ap_game_store
  WHERE (ap_game_store.id = old.id)
  RETURNING ap_game_store.id,
    ap_game_store.tracker_id,
    ap_game_store.name,
    ap_game_store.game,
    ap_game_store.checks_done,
    ap_game_store.checks_total,
    ap_game_store.last_activity,
    ap_game_store.discord_username,
    ap_game_store.last_checked,
    ap_game_store."position",
    ap_game_store.tracker_status,
    ap_game_store.notes,
    ap_game_store.discord_ping,
    ap_game_store.claimed_by_ct_user_id,
    ap_game_store.availability_status,
    ap_game_store.completion_status,
    ap_game_store.progression_status,
    COALESCE(( SELECT u.discord_username
           FROM public.ct_user u
          WHERE (u.id = ap_game_store.claimed_by_ct_user_id)), ap_game_store.discord_username) AS effective_discord_username;


--
-- Name: ap_game ap_game_insert_store; Type: RULE; Schema: public; Owner: -
--

CREATE RULE ap_game_insert_store AS
    ON INSERT TO public.ap_game DO INSTEAD  INSERT INTO public.ap_game_store (id, tracker_id, name, game, checks_done, checks_total, last_activity, discord_username, last_checked, "position", tracker_status, notes, discord_ping, claimed_by_ct_user_id, availability_status, completion_status, progression_status)
  VALUES (new.id, new.tracker_id, new.name, new.game, new.checks_done, new.checks_total, new.last_activity, new.discord_username, new.last_checked, new."position", new.tracker_status, new.notes, new.discord_ping, new.claimed_by_ct_user_id, new.availability_status, new.completion_status, new.progression_status)
  RETURNING ap_game_store.id,
    ap_game_store.tracker_id,
    ap_game_store.name,
    ap_game_store.game,
    ap_game_store.checks_done,
    ap_game_store.checks_total,
    ap_game_store.last_activity,
    ap_game_store.discord_username,
    ap_game_store.last_checked,
    ap_game_store."position",
    ap_game_store.tracker_status,
    ap_game_store.notes,
    ap_game_store.discord_ping,
    ap_game_store.claimed_by_ct_user_id,
    ap_game_store.availability_status,
    ap_game_store.completion_status,
    ap_game_store.progression_status,
    COALESCE(( SELECT u.discord_username
           FROM public.ct_user u
          WHERE (u.id = ap_game_store.claimed_by_ct_user_id)), ap_game_store.discord_username) AS effective_discord_username;


--
-- Name: ap_game ap_game_update_store; Type: RULE; Schema: public; Owner: -
--

CREATE RULE ap_game_update_store AS
    ON UPDATE TO public.ap_game DO INSTEAD  UPDATE public.ap_game_store SET id = new.id, tracker_id = new.tracker_id, name = new.name, game = new.game, checks_done = new.checks_done, checks_total = new.checks_total, last_activity = new.last_activity, discord_username = new.discord_username, last_checked = new.last_checked, "position" = new."position", tracker_status = new.tracker_status, notes = new.notes, discord_ping = new.discord_ping, claimed_by_ct_user_id = new.claimed_by_ct_user_id, availability_status = new.availability_status, completion_status = new.completion_status, progression_status = new.progression_status
  WHERE (ap_game_store.id = old.id)
  RETURNING ap_game_store.id,
    ap_game_store.tracker_id,
    ap_game_store.name,
    ap_game_store.game,
    ap_game_store.checks_done,
    ap_game_store.checks_total,
    ap_game_store.last_activity,
    ap_game_store.discord_username,
    ap_game_store.last_checked,
    ap_game_store."position",
    ap_game_store.tracker_status,
    ap_game_store.notes,
    ap_game_store.discord_ping,
    ap_game_store.claimed_by_ct_user_id,
    ap_game_store.availability_status,
    ap_game_store.completion_status,
    ap_game_store.progression_status,
    COALESCE(( SELECT u.discord_username
           FROM public.ct_user u
          WHERE (u.id = ap_game_store.claimed_by_ct_user_id)), ap_game_store.discord_username) AS effective_discord_username;


--
-- Name: ap_game_store ap_game_claimed_by_ct_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game_store
    ADD CONSTRAINT ap_game_claimed_by_ct_user_id_fkey FOREIGN KEY (claimed_by_ct_user_id) REFERENCES public.ct_user(id) ON UPDATE CASCADE ON DELETE SET NULL;


--
-- Name: ap_hint ap_hint_finder_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint
    ADD CONSTRAINT ap_hint_finder_game_id_fkey FOREIGN KEY (finder_game_id) REFERENCES public.ap_game_store(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: ap_hint ap_hint_receiver_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_hint
    ADD CONSTRAINT ap_hint_receiver_game_id_fkey FOREIGN KEY (receiver_game_id) REFERENCES public.ap_game_store(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: ap_game_store ap_tracker_game_tracker_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_game_store
    ADD CONSTRAINT ap_tracker_game_tracker_id_fkey FOREIGN KEY (tracker_id) REFERENCES public.ap_tracker(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: ap_tracker ap_tracker_owner_ct_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.ap_tracker
    ADD CONSTRAINT ap_tracker_owner_ct_user_id_fkey FOREIGN KEY (owner_ct_user_id) REFERENCES public.ct_user(id) ON UPDATE CASCADE ON DELETE SET NULL;


--
-- Name: js_error js_error_ct_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.js_error
    ADD CONSTRAINT js_error_ct_user_id_fkey FOREIGN KEY (ct_user_id) REFERENCES public.ct_user(id) ON UPDATE CASCADE ON DELETE SET NULL;


--
-- PostgreSQL database dump complete
--

