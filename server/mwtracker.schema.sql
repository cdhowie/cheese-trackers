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
    ('liberally', 'sparingly', 'hints', 'see_notes', 'none');

ALTER TABLE public.ap_game
	ALTER COLUMN discord_ping DROP DEFAULT,
	ALTER COLUMN discord_ping TYPE ping_preference USING 'never'::ping_preference,
	ALTER COLUMN discord_ping SET DEFAULT 'never'::ping_preference;
