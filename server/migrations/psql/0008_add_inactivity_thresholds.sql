ALTER TABLE ap_tracker ADD COLUMN inactivity_threshold_yellow_hours INTEGER DEFAULT 24 NOT NULL;
ALTER TABLE ap_tracker ADD COLUMN inactivity_threshold_red_hours INTEGER DEFAULT 48 NOT NULL;
