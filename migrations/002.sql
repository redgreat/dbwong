CREATE TABLE IF NOT EXISTS app_settings (
 singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
 session_hours integer NOT NULL DEFAULT 8 CHECK (session_hours BETWEEN 1 AND 168)
);
INSERT INTO app_settings(singleton) VALUES(true) ON CONFLICT DO NOTHING;
