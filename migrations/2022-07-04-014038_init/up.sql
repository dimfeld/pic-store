CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE image_format AS ENUM (
  'png',
  'jpg',
  'avif',
  'webp'
);

CREATE TABLE teams (
  id uuid primary key,
  name text not null,
  deleted timestamptz
);

CREATE TABLE projects (
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  name text not null,
  base_location text not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE TABLE conversion_profiles (
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  project_id uuid references projects(id) DEFERRABLE INITIALLY IMMEDIATE,
  name text not null,

  output jsonb not null,

  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX conversion_profiles_team_id ON conversion_profiles(team_id);

CREATE TABLE storage_locations (
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  project_id uuid references projects(id) DEFERRABLE INITIALLY IMMEDIATE,
  name text not null,
  provider jsonb not null,
  base_location text not null,
  public_url_base text not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX storage_locations_team_id ON storage_locations(team_id);

CREATE TABLE upload_profiles (
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  project_id uuid not null references projects(id) DEFERRABLE INITIALLY IMMEDIATE,
  name text not null,
  short_id text,
  base_storage_location_id uuid not null references storage_locations(id) DEFERRABLE INITIALLY IMMEDIATE,
  base_storage_location_path text,
  output_storage_location_id uuid not null references storage_locations(id) DEFERRABLE INITIALLY IMMEDIATE,
  output_storage_location_path text,
  conversion_profile_id uuid not null references conversion_profiles(id) DEFERRABLE INITIALLY IMMEDIATE,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX upload_profiles_team_id_project_id ON upload_profiles(team_id, project_id);
CREATE UNIQUE INDEX upload_profiles_short_id ON upload_profiles(team_id, short_id);

CREATE TABLE users (
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  email text not null,
  password_hash text,
  name text not null,

  default_upload_profile_id uuid references upload_profiles(id) DEFERRABLE INITIALLY IMMEDIATE,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE TABLE roles (
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  name text not null,
  created timestamptz not null default now(),
  deleted timestamptz
);

CREATE TABLE user_roles (
  user_id uuid not null references users(id) on delete cascade DEFERRABLE INITIALLY IMMEDIATE,
  role_id uuid not null references roles(id) on delete cascade DEFERRABLE INITIALLY IMMEDIATE,
  added timestamptz not null default now(),
  primary key(user_id, role_id)
);

CREATE INDEX users_team_id ON users(team_id);

CREATE TABLE sessions (
  id uuid primary key,
  user_id uuid not null references users(id) DEFERRABLE INITIALLY IMMEDIATE,
  expires timestamptz not null
);

CREATE TABLE api_keys (
  id uuid primary key,
  name text not null default '',
  prefix text not null,
  hash bytea not null,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  user_id uuid not null references users(id) DEFERRABLE INITIALLY IMMEDIATE,
  default_upload_profile_id uuid references upload_profiles(id) DEFERRABLE INITIALLY IMMEDIATE,
  inherits_user_permissions boolean not null,
  created timestamptz not null default now(),
  expires timestamptz
);

CREATE INDEX api_keys_team_id_user_id ON api_keys(team_id, user_id);

CREATE TYPE permission AS ENUM (
  'team:admin',
  'team:write',
  'project:create',
  'project:write',
  'project:read',
  'image:edit',
  'image:create',
  'conversion_profile:write',
  'storage_location:write'
);

CREATE TABLE api_key_permissions (
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  api_key_id uuid not null references api_keys(id) on delete cascade DEFERRABLE INITIALLY IMMEDIATE,
  project_id uuid not null default uuid_nil(),
  permission permission not null,
  primary key(team_id, api_key_id, project_id, permission)
);

CREATE TABLE role_permissions (
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  role_id uuid not null references roles(id) DEFERRABLE INITIALLY IMMEDIATE,
  project_id uuid not null default uuid_nil(),
  permission permission not null,
  primary key(team_id, role_id, project_id, permission)
);

CREATE INDEX role_permissions_team_id_role_id ON role_permissions(team_id, role_id);

CREATE TYPE base_image_status AS ENUM (
  'awaiting_upload',
  'converting',
  'ready',
  'queued_for_delete',
  'deleting',
  'deleted'
);

CREATE TABLE base_images (
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  project_id uuid not null references projects(id) DEFERRABLE INITIALLY IMMEDIATE,
  user_id uuid not null references users(id) DEFERRABLE INITIALLY IMMEDIATE,
  hash text,
  filename text not null,
  location text not null,
  width int not null default 0,
  height int not null default 0,
  format image_format,
  upload_profile_id uuid not null references upload_profiles(id) DEFERRABLE INITIALLY IMMEDIATE,
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
  id uuid primary key,
  team_id uuid not null references teams(id) DEFERRABLE INITIALLY IMMEDIATE,
  base_image_id uuid not null references base_images(id) DEFERRABLE INITIALLY IMMEDIATE,
  location text not null,
  width int,
  height int,
  size jsonb not null,
  format jsonb not null,
  status output_image_status not null,
  updated timestamptz not null default now(),
  deleted timestamptz
);

CREATE INDEX output_images_team_id ON output_images(team_id);
CREATE INDEX output_images_base_image_id ON output_images(base_image_id);
CREATE UNIQUE INDEX output_images_base_image_id_location ON output_images(base_image_id, location);

