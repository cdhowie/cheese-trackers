ALTER TABLE ap_tracker ADD COLUMN room_link TEXT DEFAULT '' NOT NULL;
ALTER TABLE ap_tracker ADD COLUMN last_port INTEGER;
ALTER TABLE ap_tracker ADD COLUMN next_port_check_at TIMESTAMP WITH TIME ZONE;
