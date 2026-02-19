export interface AwesomeRepo {
	owner: string;
	name: string;
	description: string;
	stars: string;
	language: string;
	tags: string[];
}

export interface Category {
	id: string;
	label: string;
	icon: string;
	repos: AwesomeRepo[];
}

export const categories: Category[] = [
	{
		id: 'frontend',
		label: 'Frontend',
		icon: '/>',
		repos: [
			{ owner: 'facebook', name: 'react', description: 'The library for web and native user interfaces', stars: '233k', language: 'JavaScript', tags: ['ui', 'framework'] },
			{ owner: 'vuejs', name: 'core', description: 'The progressive JavaScript framework', stars: '48k', language: 'TypeScript', tags: ['ui', 'framework'] },
			{ owner: 'sveltejs', name: 'svelte', description: 'Cybernetically enhanced web apps', stars: '81k', language: 'JavaScript', tags: ['ui', 'compiler'] },
			{ owner: 'angular', name: 'angular', description: 'Deliver web apps with confidence', stars: '97k', language: 'TypeScript', tags: ['ui', 'framework'] },
			{ owner: 'solidjs', name: 'solid', description: 'A declarative, efficient, and flexible JavaScript library for building UIs', stars: '33k', language: 'TypeScript', tags: ['ui', 'reactive'] },
			{ owner: 'preactjs', name: 'preact', description: 'Fast 3kB React alternative with the same modern API', stars: '37k', language: 'JavaScript', tags: ['ui', 'lightweight'] },
			{ owner: 'lit', name: 'lit', description: 'Simple. Fast. Web Components.', stars: '19k', language: 'TypeScript', tags: ['web-components', 'ui'] },
			{ owner: 'alpinejs', name: 'alpine', description: 'A rugged, minimal framework for composing behavior directly in your markup', stars: '29k', language: 'JavaScript', tags: ['ui', 'lightweight'] },
			{ owner: 'htmx-org', name: 'htmx', description: 'High power tools for HTML', stars: '42k', language: 'JavaScript', tags: ['hypermedia', 'lightweight'] },
		]
	},
	{
		id: 'meta-frameworks',
		label: 'Meta Frameworks',
		icon: '>>',
		repos: [
			{ owner: 'vercel', name: 'next.js', description: 'The React Framework', stars: '130k', language: 'JavaScript', tags: ['react', 'ssr', 'fullstack'] },
			{ owner: 'nuxt', name: 'nuxt', description: 'The Intuitive Vue Framework', stars: '56k', language: 'TypeScript', tags: ['vue', 'ssr', 'fullstack'] },
			{ owner: 'remix-run', name: 'remix', description: 'Build better websites with Remix', stars: '31k', language: 'TypeScript', tags: ['react', 'ssr'] },
			{ owner: 'withastro', name: 'astro', description: 'The web framework for content-driven websites', stars: '49k', language: 'TypeScript', tags: ['static', 'islands'] },
			{ owner: 'sveltejs', name: 'kit', description: 'Web development, streamlined', stars: '19k', language: 'JavaScript', tags: ['svelte', 'ssr'] },
			{ owner: 'analogjs', name: 'analog', description: 'The fullstack Angular meta-framework', stars: '3k', language: 'TypeScript', tags: ['angular', 'ssr'] },
			{ owner: 'gatsbyjs', name: 'gatsby', description: 'The best React-based framework for performance and scalability', stars: '55k', language: 'JavaScript', tags: ['react', 'static'] },
		]
	},
	{
		id: 'css-styling',
		label: 'CSS & Styling',
		icon: '#~',
		repos: [
			{ owner: 'tailwindlabs', name: 'tailwindcss', description: 'A utility-first CSS framework for rapid UI development', stars: '86k', language: 'CSS', tags: ['utility', 'css'] },
			{ owner: 'unocss', name: 'unocss', description: 'The instant on-demand atomic CSS engine', stars: '17k', language: 'TypeScript', tags: ['atomic', 'css'] },
			{ owner: 'styled-components', name: 'styled-components', description: 'Visual primitives for the component age', stars: '40k', language: 'TypeScript', tags: ['css-in-js', 'react'] },
			{ owner: 'open-props', name: 'open-props', description: 'CSS custom properties to help accelerate adaptive and consistent design', stars: '5k', language: 'CSS', tags: ['custom-properties', 'design'] },
			{ owner: 'picocss', name: 'pico', description: 'Minimal CSS Framework for semantic HTML', stars: '14k', language: 'SCSS', tags: ['classless', 'minimal'] },
			{ owner: 'animate-css', name: 'animate.css', description: 'A cross-browser library of CSS animations', stars: '81k', language: 'CSS', tags: ['animations', 'css'] },
		]
	},
	{
		id: 'ui-components',
		label: 'UI Components',
		icon: '[]',
		repos: [
			{ owner: 'shadcn-ui', name: 'ui', description: 'Beautifully designed components built with Radix UI and Tailwind CSS', stars: '82k', language: 'TypeScript', tags: ['react', 'tailwind'] },
			{ owner: 'radix-ui', name: 'primitives', description: 'Radix Primitives is an open-source UI component library', stars: '16k', language: 'TypeScript', tags: ['react', 'headless'] },
			{ owner: 'chakra-ui', name: 'chakra-ui', description: 'Simple, Modular & Accessible UI Components for React', stars: '38k', language: 'TypeScript', tags: ['react', 'accessible'] },
			{ owner: 'mantinedev', name: 'mantine', description: 'A fully featured React components library', stars: '27k', language: 'TypeScript', tags: ['react', 'components'] },
			{ owner: 'ariakit', name: 'ariakit', description: 'Toolkit for building accessible web apps with React', stars: '8k', language: 'TypeScript', tags: ['react', 'accessible'] },
			{ owner: 'huntabyte', name: 'shadcn-svelte', description: 'shadcn/ui but for Svelte', stars: '6k', language: 'Svelte', tags: ['svelte', 'tailwind'] },
			{ owner: 'ant-design', name: 'ant-design', description: 'An enterprise-class UI design language and React UI library', stars: '93k', language: 'TypeScript', tags: ['react', 'enterprise'] },
			{ owner: 'element-plus', name: 'element-plus', description: 'A Vue.js 3 UI Library made by Element team', stars: '25k', language: 'TypeScript', tags: ['vue', 'components'] },
		]
	},
	{
		id: 'backend',
		label: 'Backend',
		icon: '::',
		repos: [
			{ owner: 'expressjs', name: 'express', description: 'Fast, unopinionated, minimalist web framework for Node.js', stars: '66k', language: 'JavaScript', tags: ['node', 'http'] },
			{ owner: 'fastify', name: 'fastify', description: 'Fast and low overhead web framework for Node.js', stars: '33k', language: 'JavaScript', tags: ['node', 'http'] },
			{ owner: 'honojs', name: 'hono', description: 'Web framework built on Web Standards', stars: '22k', language: 'TypeScript', tags: ['edge', 'http'] },
			{ owner: 'elysiajs', name: 'elysia', description: 'Ergonomic framework for humans', stars: '12k', language: 'TypeScript', tags: ['bun', 'http'] },
			{ owner: 'nestjs', name: 'nest', description: 'A progressive Node.js framework for building server-side applications', stars: '69k', language: 'TypeScript', tags: ['node', 'enterprise'] },
			{ owner: 'django', name: 'django', description: 'The web framework for perfectionists with deadlines', stars: '82k', language: 'Python', tags: ['python', 'fullstack'] },
			{ owner: 'tiangolo', name: 'fastapi', description: 'Modern, fast, web framework for building APIs with Python', stars: '82k', language: 'Python', tags: ['python', 'async'] },
			{ owner: 'pallets', name: 'flask', description: 'The Python micro framework for building web applications', stars: '69k', language: 'Python', tags: ['python', 'micro'] },
			{ owner: 'gin-gonic', name: 'gin', description: 'Gin is a HTTP web framework written in Go', stars: '80k', language: 'Go', tags: ['go', 'http'] },
			{ owner: 'gofiber', name: 'fiber', description: 'Express-inspired web framework written in Go', stars: '35k', language: 'Go', tags: ['go', 'http'] },
			{ owner: 'labstack', name: 'echo', description: 'High performance, minimalist Go web framework', stars: '30k', language: 'Go', tags: ['go', 'http'] },
			{ owner: 'actix', name: 'actix-web', description: 'Actix Web is a powerful, pragmatic, and fast web framework for Rust', stars: '22k', language: 'Rust', tags: ['rust', 'http'] },
			{ owner: 'tokio-rs', name: 'axum', description: 'Ergonomic and modular web framework built with Tokio', stars: '20k', language: 'Rust', tags: ['rust', 'http'] },
			{ owner: 'spring-projects', name: 'spring-boot', description: 'Spring Boot helps you create production-grade Spring applications', stars: '76k', language: 'Java', tags: ['java', 'enterprise'] },
			{ owner: 'laravel', name: 'laravel', description: 'A web application framework with expressive, elegant syntax', stars: '79k', language: 'PHP', tags: ['php', 'fullstack'] },
		]
	},
	{
		id: 'databases',
		label: 'Databases',
		icon: 'db',
		repos: [
			{ owner: 'prisma', name: 'prisma', description: 'Next-generation ORM for Node.js & TypeScript', stars: '41k', language: 'TypeScript', tags: ['orm', 'typescript'] },
			{ owner: 'drizzle-team', name: 'drizzle-orm', description: 'Headless TypeScript ORM with a head', stars: '26k', language: 'TypeScript', tags: ['orm', 'typescript'] },
			{ owner: 'supabase', name: 'supabase', description: 'The open source Firebase alternative', stars: '78k', language: 'TypeScript', tags: ['postgres', 'baas'] },
			{ owner: 'pocketbase', name: 'pocketbase', description: 'Open source realtime backend in 1 file', stars: '43k', language: 'Go', tags: ['sqlite', 'baas'] },
			{ owner: 'turso-tech', name: 'libsql', description: 'libSQL is a fork of SQLite that is both open source and open contribution', stars: '13k', language: 'C', tags: ['sqlite', 'edge'] },
			{ owner: 'redis', name: 'redis', description: 'Redis is an in-memory database that persists on disk', stars: '68k', language: 'C', tags: ['cache', 'kv'] },
			{ owner: 'cockroachdb', name: 'cockroach', description: 'CockroachDB — the cloud native, distributed SQL database', stars: '30k', language: 'Go', tags: ['distributed', 'sql'] },
			{ owner: 'pingcap', name: 'tidb', description: 'TiDB is a distributed SQL database compatible with MySQL protocol', stars: '38k', language: 'Go', tags: ['distributed', 'mysql'] },
			{ owner: 'mongodb', name: 'mongo', description: 'The MongoDB Database', stars: '27k', language: 'C++', tags: ['nosql', 'document'] },
			{ owner: 'elastic', name: 'elasticsearch', description: 'Free and Open, Distributed, RESTful Search Engine', stars: '71k', language: 'Java', tags: ['search', 'analytics'] },
			{ owner: 'hasura', name: 'graphql-engine', description: 'Instant realtime GraphQL APIs on all your data', stars: '31k', language: 'TypeScript', tags: ['graphql', 'postgres'] },
		]
	},
	{
		id: 'devtools',
		label: 'Dev Tools',
		icon: '=>',
		repos: [
			{ owner: 'vitejs', name: 'vite', description: 'Next generation frontend tooling', stars: '71k', language: 'TypeScript', tags: ['bundler', 'hmr'] },
			{ owner: 'biomejs', name: 'biome', description: 'A toolchain for web projects — formatter, linter, and more', stars: '16k', language: 'Rust', tags: ['linter', 'formatter'] },
			{ owner: 'oxc-project', name: 'oxc', description: 'The JavaScript Oxidation Compiler', stars: '13k', language: 'Rust', tags: ['parser', 'linter'] },
			{ owner: 'evanw', name: 'esbuild', description: 'An extremely fast bundler for the web', stars: '38k', language: 'Go', tags: ['bundler', 'fast'] },
			{ owner: 'rolldown', name: 'rolldown', description: 'Fast Rust bundler for JavaScript with Rollup-compatible API', stars: '10k', language: 'Rust', tags: ['bundler', 'fast'] },
			{ owner: 'oven-sh', name: 'bun', description: 'Incredibly fast JavaScript runtime, bundler, test runner, and package manager', stars: '75k', language: 'Zig', tags: ['runtime', 'fast'] },
			{ owner: 'denoland', name: 'deno', description: 'A modern runtime for JavaScript and TypeScript', stars: '101k', language: 'Rust', tags: ['runtime', 'typescript'] },
			{ owner: 'gradle', name: 'gradle', description: 'Adaptable, fast automation for all', stars: '17k', language: 'Groovy', tags: ['build-tool', 'java'] },
			{ owner: 'docker', name: 'awesome-compose', description: 'Awesome Docker Compose samples', stars: '37k', language: 'Markdown', tags: ['docker', 'examples'] },
		]
	},
	{
		id: 'ai-ml',
		label: 'AI & ML',
		icon: '**',
		repos: [
			{ owner: 'huggingface', name: 'transformers', description: 'State-of-the-art Machine Learning for PyTorch, TensorFlow, and JAX', stars: '140k', language: 'Python', tags: ['nlp', 'models'] },
			{ owner: 'langchain-ai', name: 'langchain', description: 'Build context-aware reasoning applications', stars: '100k', language: 'Python', tags: ['llm', 'agents'] },
			{ owner: 'ollama', name: 'ollama', description: 'Get up and running with large language models', stars: '110k', language: 'Go', tags: ['llm', 'local'] },
			{ owner: 'ggerganov', name: 'llama.cpp', description: 'LLM inference in C/C++', stars: '75k', language: 'C++', tags: ['llm', 'inference'] },
			{ owner: 'open-webui', name: 'open-webui', description: 'User-friendly AI interface', stars: '60k', language: 'Svelte', tags: ['llm', 'ui'] },
			{ owner: 'mlc-ai', name: 'mlc-llm', description: 'Universal LLM Deployment Engine', stars: '20k', language: 'Python', tags: ['llm', 'deployment'] },
			{ owner: 'josephmisiti', name: 'awesome-machine-learning', description: 'A curated list of awesome Machine Learning frameworks and libraries', stars: '67k', language: 'Python', tags: ['reference', 'curated'] },
			{ owner: 'pytorch', name: 'pytorch', description: 'Tensors and Dynamic neural networks with strong GPU acceleration', stars: '86k', language: 'Python', tags: ['deep-learning', 'gpu'] },
			{ owner: 'tensorflow', name: 'tensorflow', description: 'An open source machine learning framework for everyone', stars: '187k', language: 'C++', tags: ['deep-learning', 'production'] },
		]
	},
	{
		id: 'devops',
		label: 'DevOps & Infra',
		icon: '$$',
		repos: [
			{ owner: 'docker', name: 'compose', description: 'Define and run multi-container applications with Docker', stars: '34k', language: 'Go', tags: ['containers', 'orchestration'] },
			{ owner: 'kubernetes', name: 'kubernetes', description: 'Production-Grade Container Orchestration', stars: '113k', language: 'Go', tags: ['containers', 'orchestration'] },
			{ owner: 'traefik', name: 'traefik', description: 'The Cloud Native Application Proxy', stars: '53k', language: 'Go', tags: ['proxy', 'edge'] },
			{ owner: 'caddyserver', name: 'caddy', description: 'Fast and extensible multi-platform HTTP/1-2-3 web server with automatic HTTPS', stars: '61k', language: 'Go', tags: ['server', 'https'] },
			{ owner: 'grafana', name: 'grafana', description: 'The open and composable observability and data visualization platform', stars: '66k', language: 'TypeScript', tags: ['monitoring', 'dashboards'] },
			{ owner: 'prometheus', name: 'prometheus', description: 'The Prometheus monitoring system and time series database', stars: '57k', language: 'Go', tags: ['monitoring', 'metrics'] },
			{ owner: 'hashicorp', name: 'terraform', description: 'Terraform enables you to safely manage infrastructure as code', stars: '43k', language: 'Go', tags: ['iac', 'cloud'] },
			{ owner: 'ansible', name: 'ansible', description: 'Radically simple IT automation', stars: '64k', language: 'Python', tags: ['automation', 'config'] },
		]
	},
	{
		id: 'rust',
		label: 'Rust',
		icon: 'Rs',
		repos: [
			{ owner: 'rust-lang', name: 'rust', description: 'Empowering everyone to build reliable and efficient software', stars: '101k', language: 'Rust', tags: ['compiler', 'systems'] },
			{ owner: 'tokio-rs', name: 'tokio', description: 'A runtime for writing reliable asynchronous applications', stars: '28k', language: 'Rust', tags: ['async', 'runtime'] },
			{ owner: 'serde-rs', name: 'serde', description: 'Serialization framework for Rust', stars: '9k', language: 'Rust', tags: ['serialization'] },
			{ owner: 'BurntSushi', name: 'ripgrep', description: 'A line-oriented search tool that recursively searches', stars: '50k', language: 'Rust', tags: ['cli', 'search'] },
			{ owner: 'sharkdp', name: 'bat', description: 'A cat clone with syntax highlighting and Git integration', stars: '51k', language: 'Rust', tags: ['cli', 'tool'] },
			{ owner: 'starship', name: 'starship', description: 'The minimal, blazing-fast, and customizable prompt for any shell', stars: '47k', language: 'Rust', tags: ['cli', 'prompt'] },
			{ owner: 'astral-sh', name: 'ruff', description: 'An extremely fast Python linter and code formatter, written in Rust', stars: '36k', language: 'Rust', tags: ['python', 'linter'] },
			{ owner: 'astral-sh', name: 'uv', description: 'An extremely fast Python package and project manager, written in Rust', stars: '35k', language: 'Rust', tags: ['python', 'package-manager'] },
			{ owner: 'paritytech', name: 'parity-ethereum', description: 'The fast, light, and robust client for Ethereum-like networks', stars: '16k', language: 'Rust', tags: ['blockchain', 'ethereum'] },
		]
	},
	{
		id: 'editors',
		label: 'Editors',
		icon: 'vi',
		repos: [
			{ owner: 'neovim', name: 'neovim', description: 'Vim-fork focused on extensibility and usability', stars: '86k', language: 'Vim Script', tags: ['terminal', 'extensible'] },
			{ owner: 'helix-editor', name: 'helix', description: 'A post-modern modal text editor', stars: '35k', language: 'Rust', tags: ['terminal', 'modal'] },
			{ owner: 'zed-industries', name: 'zed', description: 'Code at the speed of thought', stars: '55k', language: 'Rust', tags: ['gui', 'fast'] },
			{ owner: 'lapce', name: 'lapce', description: 'Lightning-fast and powerful code editor written in Rust', stars: '35k', language: 'Rust', tags: ['gui', 'fast'] },
			{ owner: 'obsidianmd', name: 'obsidian-releases', description: 'Community plugins list for Obsidian', stars: '12k', language: 'Markdown', tags: ['notes', 'plugins'] },
		]
	},
	{
		id: 'gaming',
		label: 'Game Dev',
		icon: '::',
		repos: [
			{ owner: 'godotengine', name: 'godot', description: 'Multi-platform 2D and 3D game engine', stars: '95k', language: 'C++', tags: ['engine', '2d', '3d'] },
			{ owner: 'bevyengine', name: 'bevy', description: 'A refreshingly simple data-driven game engine built in Rust', stars: '38k', language: 'Rust', tags: ['engine', 'ecs'] },
			{ owner: 'mrdoob', name: 'three.js', description: 'JavaScript 3D Library', stars: '104k', language: 'JavaScript', tags: ['webgl', '3d'] },
			{ owner: 'photonstorm', name: 'phaser', description: 'Phaser is a fun, free and fast 2D game framework for making HTML5 games', stars: '37k', language: 'JavaScript', tags: ['2d', 'browser'] },
			{ owner: 'libgdx', name: 'libgdx', description: 'Desktop/Android/HTML5/iOS Java game development framework', stars: '24k', language: 'Java', tags: ['engine', 'cross-platform'] },
		]
	},
	{
		id: 'security',
		label: 'Security',
		icon: '!!',
		repos: [
			{ owner: 'OWASP', name: 'CheatSheetSeries', description: 'Cheatsheets for application security', stars: '28k', language: 'Markdown', tags: ['reference', 'web'] },
			{ owner: 'gitleaks', name: 'gitleaks', description: 'Protect and discover secrets using Gitleaks', stars: '18k', language: 'Go', tags: ['secrets', 'scanner'] },
			{ owner: 'trufflesecurity', name: 'trufflehog', description: 'Find, verify, and analyze leaked credentials', stars: '17k', language: 'Go', tags: ['secrets', 'scanner'] },
			{ owner: 'aquasecurity', name: 'trivy', description: 'Find vulnerabilities, misconfigurations, secrets in containers and code', stars: '24k', language: 'Go', tags: ['scanner', 'containers'] },
			{ owner: 'sbilly', name: 'awesome-security', description: 'A collection of awesome software, libraries, and tools for security', stars: '13k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'qazbnm456', name: 'awesome-web-security', description: 'A curated list of web security materials and resources', stars: '11k', language: 'Markdown', tags: ['reference', 'web'] },
			{ owner: 'zeek', name: 'zeek', description: 'A powerful network analysis framework for security monitoring', stars: '7k', language: 'C++', tags: ['network', 'monitoring'] },
		]
	},
	{
		id: 'testing',
		label: 'Testing',
		icon: '??',
		repos: [
			{ owner: 'vitest-dev', name: 'vitest', description: 'Next generation testing framework powered by Vite', stars: '14k', language: 'TypeScript', tags: ['unit', 'vite'] },
			{ owner: 'playwright-community', name: 'playwright', description: 'Playwright is a framework for Web Testing and Automation', stars: '3k', language: 'TypeScript', tags: ['e2e', 'browser'] },
			{ owner: 'cypress-io', name: 'cypress', description: 'Fast, easy and reliable testing for anything that runs in a browser', stars: '48k', language: 'JavaScript', tags: ['e2e', 'browser'] },
			{ owner: 'stretchr', name: 'testify', description: 'A toolkit with common assertions and mocks for Go', stars: '24k', language: 'Go', tags: ['go', 'assertions'] },
			{ owner: 'pytest-dev', name: 'pytest', description: 'The pytest framework makes it easy to write small tests', stars: '12k', language: 'Python', tags: ['python', 'unit'] },
		]
	},
	{
		id: 'learning',
		label: 'Learning',
		icon: '>>',
		repos: [
			{ owner: 'freeCodeCamp', name: 'freeCodeCamp', description: 'Open source codebase and curriculum for learning to code', stars: '410k', language: 'TypeScript', tags: ['education', 'curriculum'] },
			{ owner: 'codecrafters-io', name: 'build-your-own-x', description: 'Master programming by recreating your favorite technologies from scratch', stars: '467k', language: 'Markdown', tags: ['education', 'projects'] },
			{ owner: 'ossu', name: 'computer-science', description: 'Path to a free self-taught education in Computer Science', stars: '201k', language: 'Markdown', tags: ['education', 'curriculum'] },
			{ owner: 'MunGell', name: 'awesome-for-beginners', description: 'A list of awesome beginners-friendly projects', stars: '83k', language: 'Markdown', tags: ['beginner', 'curated'] },
			{ owner: 'prakhar1989', name: 'awesome-courses', description: 'List of awesome university courses for learning Computer Science', stars: '67k', language: 'Markdown', tags: ['education', 'courses'] },
			{ owner: 'Chalarangelo', name: '30-seconds-of-code', description: 'Coding articles to level up your development skills', stars: '127k', language: 'JavaScript', tags: ['snippets', 'reference'] },
			{ owner: 'iluwatar', name: 'java-design-patterns', description: 'Design patterns implemented in Java', stars: '94k', language: 'Java', tags: ['patterns', 'java'] },
			{ owner: 'tayllan', name: 'awesome-algorithms', description: 'A curated list of awesome places to learn algorithms', stars: '20k', language: 'Markdown', tags: ['algorithms', 'curated'] },
			{ owner: 'trimstray', name: 'the-book-of-secret-knowledge', description: 'A collection of inspiring lists, manuals, cheatsheets, and hacks', stars: '207k', language: 'Markdown', tags: ['reference', 'sysadmin'] },
		]
	},
	{
		id: 'mobile',
		label: 'Mobile',
		icon: '##',
		repos: [
			{ owner: 'facebook', name: 'react-native', description: 'A framework for building native applications using React', stars: '121k', language: 'C++', tags: ['react', 'cross-platform'] },
			{ owner: 'flutter', name: 'flutter', description: 'Flutter makes it easy to build beautiful apps for mobile, web, and desktop', stars: '168k', language: 'Dart', tags: ['cross-platform', 'ui'] },
			{ owner: 'nicklockwood', name: 'iVersion', description: 'Library for dynamically checking for updates to Mac/iPhone App Store apps', stars: '2k', language: 'Objective-C', tags: ['ios', 'updates'] },
			{ owner: 'expo', name: 'expo', description: 'An open-source framework for making universal native apps', stars: '36k', language: 'TypeScript', tags: ['react-native', 'tooling'] },
			{ owner: 'nicklockwood', name: 'SwiftFormat', description: 'A command-line tool and Xcode extension for formatting Swift code', stars: '8k', language: 'Swift', tags: ['ios', 'formatter'] },
			{ owner: 'nicklockwood', name: 'iCarousel', description: 'A simple, highly customisable, data-driven 3D carousel for iOS', stars: '12k', language: 'Objective-C', tags: ['ios', 'ui'] },
		]
	},
	{
		id: 'cli',
		label: 'CLI Tools',
		icon: '$>',
		repos: [
			{ owner: 'junegunn', name: 'fzf', description: 'A command-line fuzzy finder', stars: '68k', language: 'Go', tags: ['fuzzy', 'search'] },
			{ owner: 'jesseduffield', name: 'lazygit', description: 'Simple terminal UI for git commands', stars: '55k', language: 'Go', tags: ['git', 'tui'] },
			{ owner: 'sharkdp', name: 'fd', description: 'A simple, fast and user-friendly alternative to find', stars: '35k', language: 'Rust', tags: ['search', 'fast'] },
			{ owner: 'ajeetdsouza', name: 'zoxide', description: 'A smarter cd command inspired by z and autojump', stars: '24k', language: 'Rust', tags: ['navigation', 'shell'] },
			{ owner: 'charmbracelet', name: 'bubbletea', description: 'A powerful little TUI framework', stars: '29k', language: 'Go', tags: ['tui', 'framework'] },
			{ owner: 'tmux', name: 'tmux', description: 'Terminal multiplexer', stars: '36k', language: 'C', tags: ['terminal', 'multiplexer'] },
			{ owner: 'ogham', name: 'exa', description: 'A modern replacement for ls', stars: '24k', language: 'Rust', tags: ['ls', 'modern'] },
			{ owner: 'jqlang', name: 'jq', description: 'Command-line JSON processor', stars: '31k', language: 'C', tags: ['json', 'parser'] },
		]
	},
	{
		id: 'self-hosted',
		label: 'Self-Hosted',
		icon: '@@',
		repos: [
			{ owner: 'awesome-selfhosted', name: 'awesome-selfhosted', description: 'A list of free software network services and web applications to self-host', stars: '274k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'immich-app', name: 'immich', description: 'High performance self-hosted photo and video management solution', stars: '58k', language: 'TypeScript', tags: ['photos', 'media'] },
			{ owner: 'nextcloud', name: 'server', description: 'Nextcloud server, a safe home for all your data', stars: '28k', language: 'PHP', tags: ['cloud', 'storage'] },
			{ owner: 'gethomepage', name: 'homepage', description: 'A highly customizable homepage with service API integrations', stars: '22k', language: 'JavaScript', tags: ['dashboard', 'monitoring'] },
			{ owner: 'louislam', name: 'uptime-kuma', description: 'A fancy self-hosted monitoring tool', stars: '63k', language: 'JavaScript', tags: ['monitoring', 'uptime'] },
			{ owner: 'odoo', name: 'odoo', description: 'Open Source Apps To Grow Your Business — ERP & CRM', stars: '40k', language: 'Python', tags: ['erp', 'business'] },
		]
	},
	{
		id: 'data-science',
		label: 'Data Science',
		icon: '~~',
		repos: [
			{ owner: 'pandas-dev', name: 'pandas', description: 'Flexible and powerful data analysis and manipulation library for Python', stars: '44k', language: 'Python', tags: ['dataframes', 'analysis'] },
			{ owner: 'apache', name: 'spark', description: 'Unified analytics engine for large-scale data processing', stars: '40k', language: 'Scala', tags: ['big-data', 'distributed'] },
			{ owner: 'apache', name: 'superset', description: 'Apache Superset is a data visualization and exploration platform', stars: '64k', language: 'TypeScript', tags: ['visualization', 'dashboards'] },
			{ owner: 'apache', name: 'airflow', description: 'Platform to programmatically author, schedule and monitor workflows', stars: '38k', language: 'Python', tags: ['workflow', 'orchestration'] },
			{ owner: 'plotly', name: 'plotly.js', description: 'Open-source JavaScript charting library', stars: '18k', language: 'JavaScript', tags: ['charts', 'visualization'] },
			{ owner: 'jupyter', name: 'notebook', description: 'Jupyter Interactive Notebook', stars: '12k', language: 'Python', tags: ['notebook', 'interactive'] },
		]
	},
	{
		id: 'design',
		label: 'Design Systems',
		icon: '()',
		repos: [
			{ owner: 'storybookjs', name: 'storybook', description: 'The UI component explorer for frontend developers', stars: '85k', language: 'TypeScript', tags: ['components', 'documentation'] },
			{ owner: 'penpot', name: 'penpot', description: 'The open source design and prototyping platform', stars: '35k', language: 'Clojure', tags: ['design', 'prototyping'] },
			{ owner: 'FortAwesome', name: 'Font-Awesome', description: 'The iconic SVG, font, and CSS toolkit', stars: '74k', language: 'JavaScript', tags: ['icons', 'svg'] },
			{ owner: 'lucide-icons', name: 'lucide', description: 'Beautiful & consistent icon toolkit made by the community', stars: '13k', language: 'TypeScript', tags: ['icons', 'svg'] },
			{ owner: 'goabstract', name: 'Awesome-Design-Tools', description: 'The best design tools and plugins for everything', stars: '34k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'alexpate', name: 'awesome-design-systems', description: 'A collection of awesome design systems', stars: '18k', language: 'Markdown', tags: ['reference', 'curated'] },
		]
	},
];

export const allLanguages = [...new Set(categories.flatMap(c => c.repos.map(r => r.language)))].sort();
export const allTags = [...new Set(categories.flatMap(c => c.repos.flatMap(r => r.tags)))].sort();
