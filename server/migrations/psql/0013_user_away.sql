ALTER TABLE ct_user ADD COLUMN is_away BOOLEAN NOT NULL DEFAULT FALSE;

CREATE OR REPLACE VIEW ap_game WITH (security_barrier='false', security_invoker='true') AS
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
    COALESCE(u.discord_username, g.discord_username) AS effective_discord_username,
    COALESCE(u.is_away, FALSE) AS user_is_away
   FROM (public.ap_game_store g
     LEFT JOIN public.ct_user u ON ((u.id = g.claimed_by_ct_user_id)));

CREATE OR REPLACE RULE ap_game_delete_store AS
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
          WHERE (u.id = ap_game_store.claimed_by_ct_user_id)), ap_game_store.discord_username) AS effective_discord_username,
    COALESCE(
        (SELECT is_away FROM ct_user u WHERE u.id = ap_game_store.claimed_by_ct_user_id),
        FALSE
    ) AS user_is_away;

CREATE OR REPLACE RULE ap_game_insert_store AS
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
          WHERE (u.id = ap_game_store.claimed_by_ct_user_id)), ap_game_store.discord_username) AS effective_discord_username,
    COALESCE(
        (SELECT is_away FROM ct_user u WHERE u.id = ap_game_store.claimed_by_ct_user_id),
        FALSE
    ) AS user_is_away;

CREATE OR REPLACE RULE ap_game_update_store AS
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
          WHERE (u.id = ap_game_store.claimed_by_ct_user_id)), ap_game_store.discord_username) AS effective_discord_username,
    COALESCE(
        (SELECT is_away FROM ct_user u WHERE u.id = ap_game_store.claimed_by_ct_user_id),
        FALSE
    ) AS user_is_away;
