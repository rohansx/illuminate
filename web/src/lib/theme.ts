export const THEMES = [
	{ id: 'forest', label: 'Forest', accent: '#73E2A7' },
	{ id: 'sunset', label: 'Sunset', accent: '#FF8811' },
	{ id: 'orchid', label: 'Orchid', accent: '#E15A97' },
	{ id: 'crimson', label: 'Crimson', accent: '#c93040' },
	{ id: 'glacier', label: 'Glacier', accent: '#64B6AC' },
	{ id: 'mono', label: 'Mono', accent: '#e0e0e0' }
] as const;

export type ThemeId = (typeof THEMES)[number]['id'];

const STORAGE_KEY = 'illuminate-theme';

export function getTheme(): ThemeId {
	if (typeof localStorage === 'undefined') return 'mono';
	return (localStorage.getItem(STORAGE_KEY) as ThemeId) || 'mono';
}

export function setTheme(id: ThemeId) {
	document.documentElement.setAttribute('data-theme', id);
	localStorage.setItem(STORAGE_KEY, id);
}

export function initTheme() {
	const saved = getTheme();
	document.documentElement.setAttribute('data-theme', saved);
}
