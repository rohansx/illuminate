const API_BASE = 'http://localhost:8080';

class ApiError extends Error {
	status: number;
	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		credentials: 'include',
		headers: { 'Content-Type': 'application/json', ...options.headers },
		...options
	});

	if (res.status === 401) {
		// Try refresh
		const refreshRes = await fetch(`${API_BASE}/auth/refresh`, {
			method: 'POST',
			credentials: 'include'
		});

		if (refreshRes.ok) {
			// Retry original request
			const retryRes = await fetch(`${API_BASE}${path}`, {
				credentials: 'include',
				headers: { 'Content-Type': 'application/json', ...options.headers },
				...options
			});
			if (!retryRes.ok) {
				throw new ApiError(retryRes.status, 'Request failed after refresh');
			}
			return retryRes.json();
		}

		// Redirect to login
		window.location.href = `${API_BASE}/auth/github/login`;
		throw new ApiError(401, 'Unauthorized');
	}

	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: 'Unknown error' }));
		throw new ApiError(res.status, body.error || 'Request failed');
	}

	return res.json();
}

export interface User {
	id: string;
	github_id: number;
	github_username: string;
	avatar_url: string;
	bio: string;
	comfort_level: string;
	time_commitment: string;
	goals: string[];
	onboarding_done: boolean;
	skills: UserSkill[];
}

export interface UserSkill {
	language: string;
	proficiency: number;
	source: string;
}

export interface UserProfile {
	comfort_level: string;
	time_commitment: string;
	goals: string[];
}

export interface Issue {
	id: string;
	github_id: number;
	repo_id: string;
	number: number;
	title: string;
	body: string;
	summary: string;
	labels: string[];
	difficulty: number;
	time_estimate: string;
	status: string;
	comment_count: number;
	freshness_score: number;
	match_score?: number;
	match_reasons?: string[];
	repo?: Repository;
	skills?: IssueSkill[];
}

export interface Repository {
	id: string;
	owner: string;
	name: string;
	description: string;
	stars: number;
	primary_language: string;
	topics: string[];
	health_score: number;
}

export interface IssueSkill {
	language: string;
	framework?: string;
}

export interface IssueFeed {
	issues: Issue[];
	total_count: number;
	page: number;
	per_page: number;
}

export const api = {
	getMe: () => request<User>('/api/users/me'),

	updateProfile: (profile: UserProfile) =>
		request('/api/users/me/profile', {
			method: 'PATCH',
			body: JSON.stringify(profile)
		}),

	getFeed: (page = 1, perPage = 20, languages?: string[]) => {
		const params = new URLSearchParams({ page: String(page), per_page: String(perPage) });
		if (languages?.length) params.set('languages', languages.join(','));
		return request<IssueFeed>(`/api/issues/feed?${params}`);
	},

	getIssue: (id: string) => request<Issue>(`/api/issues/${id}`),

	searchIssues: (query: string, page = 1, perPage = 20) => {
		const params = new URLSearchParams({ q: query, page: String(page), per_page: String(perPage) });
		return request<IssueFeed>(`/api/issues/search?${params}`);
	},

	logout: () =>
		request('/auth/logout', { method: 'POST' }),

	loginURL: `${API_BASE}/auth/github/login`
};
