-- Consider "released" slots to be finished in addition to "goal" and "done"
-- when deciding which trackers to show on the dashboard.
DROP FUNCTION get_dashboard_trackers;

CREATE FUNCTION get_dashboard_trackers(uid integer)
RETURNS TABLE(id integer, tracker_id uuid, title text, owner_ct_user_id integer, owner_discord_username text, last_activity timestamp with time zone)
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
$$;
