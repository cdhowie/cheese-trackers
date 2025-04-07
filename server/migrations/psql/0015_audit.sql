CREATE TABLE audit (
    id SERIAL NOT NULL PRIMARY KEY,
    entity TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    changed_at TIMESTAMP WITH TIME ZONE NOT NULL,
    actor_ipaddr inet NULL,
    actor_ct_user_id INTEGER NULL REFERENCES ct_user (id) ON DELETE CASCADE ON UPDATE CASCADE,
    diff TEXT NOT NULL
);

CREATE INDEX idx_audit_entity_lookup ON audit (entity, entity_id, changed_at);

CREATE INDEX idx_audit_actor_lookup ON audit (actor_ct_user_id, changed_at);

CREATE INDEX idx_audit_actor_ip_lookup ON audit (actor_ipaddr, changed_at);

CREATE INDEX idx_audit_changed_at ON audit (changed_at);
