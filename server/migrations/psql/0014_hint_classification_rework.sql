ALTER TYPE hint_classification RENAME VALUE 'unknown' TO 'unset';
ALTER TYPE hint_classification RENAME VALUE 'critical' TO 'progression';
ALTER TYPE hint_classification RENAME VALUE 'useful' TO 'qol';
ALTER TYPE hint_classification ADD VALUE 'unknown';
ALTER TYPE hint_classification ADD VALUE 'critical';

ALTER TABLE ap_hint ALTER COLUMN classification SET DEFAULT 'unset'::hint_classification;
