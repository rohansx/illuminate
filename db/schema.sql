\restrict dbmate

-- Dumped from database version 17.7
-- Dumped by pg_dump version 18.1

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: categories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.categories (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name character varying(50) NOT NULL,
    slug character varying(50) NOT NULL,
    description text,
    icon character varying(50),
    created_at timestamp without time zone DEFAULT now() NOT NULL
);


--
-- Name: issue_skills; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.issue_skills (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    issue_id uuid NOT NULL,
    language character varying(100) NOT NULL,
    framework character varying(100) DEFAULT ''::character varying NOT NULL
);


--
-- Name: issues; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.issues (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    github_id bigint NOT NULL,
    repo_id uuid NOT NULL,
    number integer NOT NULL,
    title text NOT NULL,
    body text DEFAULT ''::text NOT NULL,
    summary text DEFAULT ''::text NOT NULL,
    labels text[] DEFAULT '{}'::text[] NOT NULL,
    difficulty smallint DEFAULT 0 NOT NULL,
    time_estimate character varying(50) DEFAULT ''::character varying NOT NULL,
    status character varying(20) DEFAULT 'open'::character varying NOT NULL,
    comment_count integer DEFAULT 0 NOT NULL,
    freshness_score real DEFAULT 0.0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    indexed_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: refresh_tokens; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.refresh_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    token_hash bytea NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: repo_categories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.repo_categories (
    repo_id uuid NOT NULL,
    category_id uuid NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL
);


--
-- Name: repositories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.repositories (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    github_id bigint NOT NULL,
    owner character varying(255) NOT NULL,
    name character varying(255) NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    stars integer DEFAULT 0 NOT NULL,
    primary_language character varying(100) DEFAULT ''::character varying NOT NULL,
    topics text[] DEFAULT '{}'::text[] NOT NULL,
    has_contributing boolean DEFAULT false NOT NULL,
    health_score real DEFAULT 0.0 NOT NULL,
    last_commit_at timestamp with time zone,
    indexed_at timestamp with time zone DEFAULT now() NOT NULL,
    tags text[] DEFAULT '{}'::text[],
    difficulty_level character varying(20) DEFAULT 'intermediate'::character varying,
    activity_status character varying(20) DEFAULT 'active'::character varying
);


--
-- Name: schema_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.schema_migrations (
    version character varying NOT NULL
);


--
-- Name: user_skills; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_skills (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    language character varying(100) NOT NULL,
    proficiency real DEFAULT 0.0 NOT NULL,
    source character varying(50) DEFAULT 'github'::character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    github_id bigint NOT NULL,
    github_username character varying(255) NOT NULL,
    avatar_url text DEFAULT ''::text NOT NULL,
    bio text DEFAULT ''::text NOT NULL,
    access_token_enc bytea,
    comfort_level character varying(20) DEFAULT 'beginner'::character varying NOT NULL,
    time_commitment character varying(50) DEFAULT ''::character varying NOT NULL,
    goals text[] DEFAULT '{}'::text[] NOT NULL,
    onboarding_done boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    role character varying(20) DEFAULT 'user'::character varying NOT NULL
);


--
-- Name: categories categories_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_name_key UNIQUE (name);


--
-- Name: categories categories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_pkey PRIMARY KEY (id);


--
-- Name: categories categories_slug_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_slug_key UNIQUE (slug);


--
-- Name: issue_skills issue_skills_issue_id_language_framework_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.issue_skills
    ADD CONSTRAINT issue_skills_issue_id_language_framework_key UNIQUE (issue_id, language, framework);


--
-- Name: issue_skills issue_skills_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.issue_skills
    ADD CONSTRAINT issue_skills_pkey PRIMARY KEY (id);


--
-- Name: issues issues_github_id_repo_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.issues
    ADD CONSTRAINT issues_github_id_repo_id_key UNIQUE (github_id, repo_id);


--
-- Name: issues issues_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.issues
    ADD CONSTRAINT issues_pkey PRIMARY KEY (id);


--
-- Name: refresh_tokens refresh_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_pkey PRIMARY KEY (id);


--
-- Name: repo_categories repo_categories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.repo_categories
    ADD CONSTRAINT repo_categories_pkey PRIMARY KEY (repo_id, category_id);


--
-- Name: repositories repositories_github_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.repositories
    ADD CONSTRAINT repositories_github_id_key UNIQUE (github_id);


--
-- Name: repositories repositories_owner_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.repositories
    ADD CONSTRAINT repositories_owner_name_key UNIQUE (owner, name);


--
-- Name: repositories repositories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.repositories
    ADD CONSTRAINT repositories_pkey PRIMARY KEY (id);


--
-- Name: schema_migrations schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.schema_migrations
    ADD CONSTRAINT schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: user_skills user_skills_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_skills
    ADD CONSTRAINT user_skills_pkey PRIMARY KEY (id);


--
-- Name: user_skills user_skills_user_id_language_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_skills
    ADD CONSTRAINT user_skills_user_id_language_key UNIQUE (user_id, language);


--
-- Name: users users_github_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_github_id_key UNIQUE (github_id);


--
-- Name: users users_github_username_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_github_username_key UNIQUE (github_username);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: idx_issue_skills_issue_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_issue_skills_issue_id ON public.issue_skills USING btree (issue_id);


--
-- Name: idx_issue_skills_language; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_issue_skills_language ON public.issue_skills USING btree (language);


--
-- Name: idx_issues_freshness; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_issues_freshness ON public.issues USING btree (freshness_score DESC);


--
-- Name: idx_issues_labels; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_issues_labels ON public.issues USING gin (labels);


--
-- Name: idx_issues_repo_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_issues_repo_id ON public.issues USING btree (repo_id);


--
-- Name: idx_issues_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_issues_status ON public.issues USING btree (status);


--
-- Name: idx_refresh_tokens_token_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_refresh_tokens_token_hash ON public.refresh_tokens USING btree (token_hash);


--
-- Name: idx_refresh_tokens_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_refresh_tokens_user_id ON public.refresh_tokens USING btree (user_id);


--
-- Name: idx_repo_categories_category; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_repo_categories_category ON public.repo_categories USING btree (category_id);


--
-- Name: idx_repo_categories_repo; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_repo_categories_repo ON public.repo_categories USING btree (repo_id);


--
-- Name: idx_repositories_activity; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_repositories_activity ON public.repositories USING btree (activity_status);


--
-- Name: idx_repositories_difficulty; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_repositories_difficulty ON public.repositories USING btree (difficulty_level);


--
-- Name: idx_repositories_health_score; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_repositories_health_score ON public.repositories USING btree (health_score);


--
-- Name: idx_repositories_primary_language; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_repositories_primary_language ON public.repositories USING btree (primary_language);


--
-- Name: idx_repositories_tags; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_repositories_tags ON public.repositories USING gin (tags);


--
-- Name: idx_user_skills_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_user_skills_user_id ON public.user_skills USING btree (user_id);


--
-- Name: idx_users_github_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_users_github_id ON public.users USING btree (github_id);


--
-- Name: issue_skills issue_skills_issue_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.issue_skills
    ADD CONSTRAINT issue_skills_issue_id_fkey FOREIGN KEY (issue_id) REFERENCES public.issues(id) ON DELETE CASCADE;


--
-- Name: issues issues_repo_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.issues
    ADD CONSTRAINT issues_repo_id_fkey FOREIGN KEY (repo_id) REFERENCES public.repositories(id) ON DELETE CASCADE;


--
-- Name: refresh_tokens refresh_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: repo_categories repo_categories_category_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.repo_categories
    ADD CONSTRAINT repo_categories_category_id_fkey FOREIGN KEY (category_id) REFERENCES public.categories(id) ON DELETE CASCADE;


--
-- Name: repo_categories repo_categories_repo_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.repo_categories
    ADD CONSTRAINT repo_categories_repo_id_fkey FOREIGN KEY (repo_id) REFERENCES public.repositories(id) ON DELETE CASCADE;


--
-- Name: user_skills user_skills_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_skills
    ADD CONSTRAINT user_skills_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict dbmate


--
-- Dbmate schema migrations
--

INSERT INTO public.schema_migrations (version) VALUES
    ('20260212000001'),
    ('20260212000002'),
    ('20260212000003'),
    ('20260212000004'),
    ('20260212000005'),
    ('20260212000006'),
    ('20260213000001'),
    ('20260213000002');
