DROP FUNCTION get_dashboard_trackers;

CREATE FUNCTION get_dashboard_trackers(uid integer)
RETURNS TABLE(
    id integer,
    tracker_id uuid,
    title text,
    owner_ct_user_id integer,
    owner_discord_username text,
    last_activity timestamp with time zone,
    dashboard_override_visibility boolean,
    room_link text,
    last_port integer,
    next_port_check_at timestamp with time zone
)
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

        UNION

        SELECT ap_tracker_id
        FROM ap_tracker_dashboard_override
        WHERE ct_user_id = uid AND visibility
    )

    SELECT
        t.id,
        t.tracker_id,
        t.title,
        t.owner_ct_user_id,
        u.discord_username AS owner_discord_username,
        gs.last_activity,
        tdo.visibility AS dashboard_override_visibility,
        t.room_link,
        t.last_port,
        t.next_port_check_at

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

    LEFT OUTER JOIN ap_tracker_dashboard_override tdo
        ON tdo.ct_user_id = uid AND tdo.ap_tracker_id = t.id

    WHERE (
        (
            t.id IN (SELECT id FROM ut)
            AND NOT gs.all_done
        ) OR tdo.visibility IS NOT DISTINCT FROM TRUE
    )
    AND tdo.visibility IS DISTINCT FROM FALSE
$$;
