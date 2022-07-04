CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE image_format AS ENUM (
  'png',
  'jpg',
  'avif',
  'webp'
);

CREATE TABLE teams (
  team_id uuid primary key,
  name text not null,
  deleted timestamptz
);

CREATE TABLE users (
  user_id uuid primary key,
  team_id uuid not null references teams(team_id),
  email text not null,
  name text not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX users_team_id ON users(team_id);

CREATE TABLE projects (
  project_id uuid primary key,
  team_id uuid not null references teams(team_id),
  name text not null,
  base_location text not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE TABLE conversion_profiles (
  conversion_profile_id uuid primary key,
  team_id uuid not null references teams(team_id),
  name text not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX conversion_profiles_team_id ON conversion_profiles(team_id);

CREATE TABLE conversion_profile_items (
  conversion_profile_item_id uuid primary key,
  conversion_profile_id uuid not null references conversion_profiles(conversion_profile_id),
  team_id uuid not null references teams(team_id),
  name text not null,
  format image_format not null,
  width int not null,
  height int not null
);

CREATE INDEX conversion_profile_items_team_id ON conversion_profile_items(team_id);
CREATE INDEX conversion_profile_items_conversion_profile_id ON conversion_profile_items(conversion_profile_id);

CREATE TABLE storage_locations (
  storage_location_id uuid primary key,
  team_id uuid not null references teams(team_id),
  name text not null,
  provider jsonb not null,
  base_location text not null,
  public_url_base text not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX storage_locations_team_id ON storage_locations(team_id);

CREATE TABLE upload_profiles (
  upload_profile_id uuid primary key,
  team_id uuid not null references teams(team_id),
  project_id uuid not null references projects(project_id),
  name text not null,
  short_id text,
  base_storage_location_id uuid not null references storage_locations(storage_location_id),
  output_storage_location_id uuid not null references storage_locations(storage_location_id),
  conversion_profile_id uuid not null references conversion_profiles(conversion_profile_id),
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX upload_profiles_team_id_project_id ON upload_profiles(team_id, project_id);
CREATE INDEX upload_profiles_short_id ON upload_profiles(team_id, short_id);

CREATE TYPE base_image_status AS ENUM (
  'awaiting_upload',
  'converting',
  'ready',
  'queued_for_delete',
  'deleting',
  'deleted'
);
CREATE TABLE base_images (
  base_image_id uuid primary key,
  team_id uuid not null references teams(team_id),
  project_id uuid not null references projects(project_id),
  user_id uuid not null references users(user_id),
  hash text,
  filename text not null,
  location text not null,
  width int not null default 0,
  height int not null default 0,
  format image_format,
  upload_profile_id uuid not null references upload_profiles(upload_profile_id),
  status base_image_status not null,
  alt_text text not null,
  placeholder text,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX base_images_team_id_project_id ON base_images(team_id, project_id);
CREATE INDEX base_images_team_id_user_id ON base_images(team_id, user_id);

CREATE TYPE output_image_status AS ENUM (
  'queued',
  'converting',
  'ready',
  'queued_for_delete',
  'deleted'
);

CREATE TABLE output_images (
  output_image_id uuid primary key,
  team_id uuid not null references teams(team_id),
  base_image_id uuid not null references base_images(base_image_id),
  location text not null,
  width int not null,
  height int not null,
  format image_format not null,
  conversion_profile_item_id uuid not null references conversion_profile_items(conversion_profile_item_id),
  status output_image_status not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX output_images_team_id ON output_images(team_id);
CREATE INDEX output_images_base_image_id ON output_images(base_image_id);
