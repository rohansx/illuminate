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
			{ owner: 'marko-js', name: 'marko', description: 'A declarative, HTML-based language that makes building web apps fun', stars: '13k', language: 'JavaScript', tags: ['ui', 'streaming'] },
			{ owner: 'hotwired', name: 'turbo', description: 'The speed of a single-page web application without having to write any JavaScript', stars: '7k', language: 'TypeScript', tags: ['html', 'lightweight'] },
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
			{ owner: 'payloadcms', name: 'payload', description: 'The best way to build a modern backend + admin UI in Next.js', stars: '30k', language: 'TypeScript', tags: ['cms', 'nextjs'] },
			{ owner: 'redwoodjs', name: 'redwood', description: 'The App Framework for Startups — built on React, GraphQL, Prisma', stars: '17k', language: 'TypeScript', tags: ['react', 'fullstack'] },
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
			{ owner: 'AllThingsSmitty', name: 'css-protips', description: 'A collection of tips to help take your CSS skills professional', stars: '30k', language: 'CSS', tags: ['tips', 'reference'] },
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
			{ owner: 'brillout', name: 'awesome-react-components', description: 'Curated list of React components and libraries', stars: '47k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'vuetifyjs', name: 'vuetify', description: 'Vue Component Framework — Material Design', stars: '40k', language: 'TypeScript', tags: ['vue', 'material'] },
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
			{ owner: 'strapi', name: 'strapi', description: 'Open-source Node.js Headless CMS to easily build customisable APIs', stars: '65k', language: 'TypeScript', tags: ['cms', 'node'] },
			{ owner: 'directus', name: 'directus', description: 'Turn any SQL database into an API and admin panel', stars: '29k', language: 'TypeScript', tags: ['cms', 'api'] },
			{ owner: 'Kong', name: 'kong', description: 'Cloud-native API gateway for microservices', stars: '40k', language: 'Lua', tags: ['api-gateway', 'cloud-native'] },
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
			{ owner: 'ClickHouse', name: 'ClickHouse', description: 'Fast open-source OLAP database management system for real-time analytics', stars: '39k', language: 'C++', tags: ['olap', 'analytics'] },
			{ owner: 'duckdb', name: 'duckdb', description: 'In-process SQL OLAP database management system', stars: '27k', language: 'C++', tags: ['olap', 'embedded'] },
			{ owner: 'questdb', name: 'questdb', description: 'High-performance time-series database for real-time analytics', stars: '15k', language: 'Java', tags: ['time-series', 'analytics'] },
			{ owner: 'timescale', name: 'timescaledb', description: 'Open-source time-series SQL database optimized for fast ingest', stars: '18k', language: 'C', tags: ['time-series', 'postgres'] },
			{ owner: 'vitessio', name: 'vitess', description: 'Database clustering system for horizontal scaling of MySQL', stars: '19k', language: 'Go', tags: ['mysql', 'sharding'] },
			{ owner: 'weaviate', name: 'weaviate', description: 'Open-source vector database for AI-native applications', stars: '12k', language: 'Go', tags: ['vector', 'ai'] },
			{ owner: 'qdrant', name: 'qdrant', description: 'High-performance vector search engine for next-gen AI', stars: '22k', language: 'Rust', tags: ['vector', 'search'] },
			{ owner: 'milvus-io', name: 'milvus', description: 'Cloud-native vector database for scalable similarity search', stars: '32k', language: 'Go', tags: ['vector', 'ai'] },
			{ owner: 'dragonflydb', name: 'dragonfly', description: 'Modern in-memory datastore, compatible with Redis and Memcached APIs', stars: '27k', language: 'C++', tags: ['cache', 'redis-compatible'] },
			{ owner: 'valkey-io', name: 'valkey', description: 'High-performance data structure server forked from Redis', stars: '18k', language: 'C', tags: ['cache', 'kv'] },
			{ owner: 'YugaByte', name: 'yugabyte-db', description: 'Cloud native distributed SQL database for mission-critical apps', stars: '9k', language: 'C', tags: ['distributed', 'sql'] },
			{ owner: 'scylladb', name: 'scylladb', description: 'NoSQL data store, Cassandra-compatible at 10x the throughput', stars: '14k', language: 'C++', tags: ['nosql', 'distributed'] },
			{ owner: 'minio', name: 'minio', description: 'High-performance object storage for AI and cloud-native', stars: '50k', language: 'Go', tags: ['object-storage', 's3'] },
			{ owner: 'seaweedfs', name: 'seaweedfs', description: 'Fast distributed storage system for billions of files', stars: '23k', language: 'Go', tags: ['storage', 'distributed'] },
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
			{ owner: 'SonarSource', name: 'SonarQube', description: 'Continuous inspection for code quality and security', stars: '9k', language: 'Java', tags: ['code-quality', 'analysis'] },
			{ owner: 'backstage', name: 'backstage', description: 'Open platform for building developer portals, created by Spotify', stars: '29k', language: 'TypeScript', tags: ['developer-portal', 'platform'] },
			{ owner: 'infisical', name: 'infisical', description: 'Open-source secrets management platform', stars: '16k', language: 'TypeScript', tags: ['secrets', 'security'] },
			{ owner: 'npm', name: 'cli', description: 'The package manager for JavaScript', stars: '9k', language: 'JavaScript', tags: ['package-manager', 'node'] },
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
			{ owner: 'langfuse', name: 'langfuse', description: 'Open-source LLM engineering platform for observability and analytics', stars: '8k', language: 'TypeScript', tags: ['llm', 'observability'] },
			{ owner: 'mindsdb', name: 'mindsdb', description: 'Platform for building AI from enterprise data', stars: '27k', language: 'Python', tags: ['ml', 'data'] },
			{ owner: 'BerriAI', name: 'litellm', description: 'Call 100+ LLM APIs in the OpenAI format', stars: '15k', language: 'Python', tags: ['llm', 'api-gateway'] },
			{ owner: 'f', name: 'awesome-chatgpt-prompts', description: 'Curated ChatGPT prompts for better results', stars: '146k', language: 'Markdown', tags: ['llm', 'reference'] },
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
			{ owner: 'argoproj', name: 'argo-cd', description: 'Declarative continuous deployment for Kubernetes', stars: '18k', language: 'Go', tags: ['gitops', 'kubernetes'] },
			{ owner: 'helm', name: 'helm', description: 'The Kubernetes Package Manager', stars: '27k', language: 'Go', tags: ['kubernetes', 'packages'] },
			{ owner: 'hashicorp', name: 'vault', description: 'Tool for secrets management and data protection', stars: '32k', language: 'Go', tags: ['secrets', 'security'] },
			{ owner: 'hashicorp', name: 'consul', description: 'Service networking solution to connect and secure services', stars: '29k', language: 'Go', tags: ['service-mesh', 'networking'] },
			{ owner: 'cilium', name: 'cilium', description: 'eBPF-based networking, observability, and security for cloud native', stars: '21k', language: 'Go', tags: ['ebpf', 'networking'] },
			{ owner: 'linkerd', name: 'linkerd2', description: 'Ultralight service mesh for Kubernetes', stars: '11k', language: 'Go', tags: ['service-mesh', 'kubernetes'] },
			{ owner: 'dapr', name: 'dapr', description: 'Portable, event-driven runtime for building distributed applications', stars: '24k', language: 'Go', tags: ['distributed', 'runtime'] },
			{ owner: 'open-policy-agent', name: 'opa', description: 'Open Policy Agent for unified policy enforcement', stars: '10k', language: 'Go', tags: ['policy', 'security'] },
			{ owner: 'cert-manager', name: 'cert-manager', description: 'Automatically provision and manage TLS certificates in Kubernetes', stars: '13k', language: 'Go', tags: ['kubernetes', 'tls'] },
		]
	},
	{
		id: 'observability',
		label: 'Observability',
		icon: '~~',
		repos: [
			{ owner: 'getsentry', name: 'sentry', description: 'Developer-first error tracking and performance monitoring', stars: '40k', language: 'Python', tags: ['error-tracking', 'apm'] },
			{ owner: 'signoz', name: 'signoz', description: 'Open-source APM, alternative to DataDog and New Relic', stars: '20k', language: 'TypeScript', tags: ['apm', 'tracing'] },
			{ owner: 'jaegertracing', name: 'jaeger', description: 'Distributed tracing platform for monitoring microservices', stars: '21k', language: 'Go', tags: ['tracing', 'distributed'] },
			{ owner: 'open-telemetry', name: 'opentelemetry-collector', description: 'Vendor-agnostic telemetry data receiver, processor and exporter', stars: '5k', language: 'Go', tags: ['telemetry', 'collector'] },
			{ owner: 'vectordotdev', name: 'vector', description: 'High-performance observability data pipeline for logs and metrics', stars: '18k', language: 'Rust', tags: ['pipeline', 'logs'] },
			{ owner: 'victoriametrics', name: 'VictoriaMetrics', description: 'Fast, cost-effective monitoring solution and time series database', stars: '13k', language: 'Go', tags: ['metrics', 'prometheus'] },
			{ owner: 'uptrace', name: 'uptrace', description: 'Open-source APM with distributed tracing and metrics', stars: '4k', language: 'Go', tags: ['apm', 'tracing'] },
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
			{ owner: 'rust-embedded', name: 'awesome-embedded-rust', description: 'Curated list of resources for embedded Rust development', stars: '6k', language: 'Rust', tags: ['embedded', 'curated'] },
			{ owner: 'TaKO8Ki', name: 'awesome-alternatives-in-rust', description: 'Curated list of CLI replacements written in Rust', stars: '4k', language: 'Markdown', tags: ['reference', 'curated'] },
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
			{ owner: 'JetBrains', name: 'intellij-community', description: 'IntelliJ IDEA Community Edition & IntelliJ Platform', stars: '17k', language: 'Java', tags: ['ide', 'java'] },
			{ owner: 'viatsko', name: 'awesome-vscode', description: 'Curated list of delightful VS Code packages and resources', stars: '28k', language: 'Markdown', tags: ['reference', 'curated'] },
		]
	},
	{
		id: 'gaming',
		label: 'Game Dev',
		icon: 'gx',
		repos: [
			{ owner: 'godotengine', name: 'godot', description: 'Multi-platform 2D and 3D game engine', stars: '95k', language: 'C++', tags: ['engine', '2d', '3d'] },
			{ owner: 'bevyengine', name: 'bevy', description: 'A refreshingly simple data-driven game engine built in Rust', stars: '38k', language: 'Rust', tags: ['engine', 'ecs'] },
			{ owner: 'mrdoob', name: 'three.js', description: 'JavaScript 3D Library', stars: '104k', language: 'JavaScript', tags: ['webgl', '3d'] },
			{ owner: 'photonstorm', name: 'phaser', description: 'Phaser is a fun, free and fast 2D game framework for making HTML5 games', stars: '37k', language: 'JavaScript', tags: ['2d', 'browser'] },
			{ owner: 'libgdx', name: 'libgdx', description: 'Desktop/Android/HTML5/iOS Java game development framework', stars: '24k', language: 'Java', tags: ['engine', 'cross-platform'] },
			{ owner: 'dawdle-deer', name: 'awesome-learn-gamedev', description: 'Curated collection of game development learning resources', stars: '4k', language: 'Markdown', tags: ['reference', 'curated'] },
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
			{ owner: 'snyk', name: 'cli', description: 'Find and fix known vulnerabilities in dependencies', stars: '5k', language: 'TypeScript', tags: ['vulnerabilities', 'sca'] },
			{ owner: 'anchore', name: 'grype', description: 'Vulnerability scanner for container images and filesystems', stars: '9k', language: 'Go', tags: ['scanner', 'sbom'] },
			{ owner: 'anchore', name: 'syft', description: 'CLI tool for generating Software Bill of Materials (SBOM)', stars: '6k', language: 'Go', tags: ['sbom', 'supply-chain'] },
			{ owner: 'sigstore', name: 'cosign', description: 'Container signing and verification tool', stars: '5k', language: 'Go', tags: ['signing', 'supply-chain'] },
		]
	},
	{
		id: 'testing',
		label: 'Testing',
		icon: '??',
		repos: [
			{ owner: 'vitest-dev', name: 'vitest', description: 'Next generation testing framework powered by Vite', stars: '14k', language: 'TypeScript', tags: ['unit', 'vite'] },
			{ owner: 'microsoft', name: 'playwright', description: 'Playwright is a framework for Web Testing and Automation', stars: '70k', language: 'TypeScript', tags: ['e2e', 'browser'] },
			{ owner: 'cypress-io', name: 'cypress', description: 'Fast, easy and reliable testing for anything that runs in a browser', stars: '48k', language: 'JavaScript', tags: ['e2e', 'browser'] },
			{ owner: 'stretchr', name: 'testify', description: 'A toolkit with common assertions and mocks for Go', stars: '24k', language: 'Go', tags: ['go', 'assertions'] },
			{ owner: 'pytest-dev', name: 'pytest', description: 'The pytest framework makes it easy to write small tests', stars: '12k', language: 'Python', tags: ['python', 'unit'] },
			{ owner: 'grafana', name: 'k6', description: 'Modern load testing tool using Go and JavaScript', stars: '26k', language: 'Go', tags: ['load-testing', 'performance'] },
			{ owner: 'SeleniumHQ', name: 'selenium', description: 'Browser automation framework and ecosystem', stars: '31k', language: 'Java', tags: ['e2e', 'browser'] },
		]
	},
	{
		id: 'learning',
		label: 'Learning',
		icon: 'ab',
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
			{ owner: 'DopplerHQ', name: 'awesome-interview-questions', description: 'A curated list of lists of interview questions', stars: '81k', language: 'Markdown', tags: ['interviews', 'reference'] },
			{ owner: 'ripienaar', name: 'free-for-dev', description: 'A list of SaaS, PaaS and IaaS offerings that have free tiers', stars: '118k', language: 'Markdown', tags: ['free-tier', 'reference'] },
			{ owner: 'binhnguyennus', name: 'awesome-scalability', description: 'The Patterns of Scalable, Reliable, and Performant Large-Scale Systems', stars: '69k', language: 'Markdown', tags: ['architecture', 'reference'] },
			{ owner: 'DovAmir', name: 'awesome-design-patterns', description: 'Software and architecture related design patterns', stars: '46k', language: 'Markdown', tags: ['patterns', 'architecture'] },
			{ owner: 'dypsilon', name: 'frontend-dev-bookmarks', description: 'Manually curated collection of resources for frontend web developers', stars: '46k', language: 'Markdown', tags: ['frontend', 'reference'] },
			{ owner: 'cloudcommunity', name: 'Free-Certifications', description: 'A curated list of free courses with certifications', stars: '51k', language: 'Markdown', tags: ['certifications', 'free'] },
			{ owner: 'PanXProject', name: 'awesome-certificates', description: 'Curated list of 20k+ hours of free courses with certificates', stars: '5k', language: 'Markdown', tags: ['certifications', 'free'] },
			{ owner: 'codecombat', name: 'codecombat', description: 'Game for learning how to code', stars: '8k', language: 'JavaScript', tags: ['education', 'game'] },
			{ owner: 'openedx', name: 'edx-platform', description: 'The Open edX LMS & Studio, powering education sites worldwide', stars: '7k', language: 'Python', tags: ['education', 'lms'] },
		]
	},
	{
		id: 'mobile',
		label: 'Mobile',
		icon: '##',
		repos: [
			{ owner: 'facebook', name: 'react-native', description: 'A framework for building native applications using React', stars: '121k', language: 'C++', tags: ['react', 'cross-platform'] },
			{ owner: 'flutter', name: 'flutter', description: 'Flutter makes it easy to build beautiful apps for mobile, web, and desktop', stars: '168k', language: 'Dart', tags: ['cross-platform', 'ui'] },
			{ owner: 'expo', name: 'expo', description: 'An open-source framework for making universal native apps', stars: '36k', language: 'TypeScript', tags: ['react-native', 'tooling'] },
			{ owner: 'nicklockwood', name: 'SwiftFormat', description: 'A command-line tool and Xcode extension for formatting Swift code', stars: '8k', language: 'Swift', tags: ['ios', 'formatter'] },
			{ owner: 'nicklockwood', name: 'iCarousel', description: 'A simple, highly customisable, data-driven 3D carousel for iOS', stars: '12k', language: 'Objective-C', tags: ['ios', 'ui'] },
			{ owner: 'jondot', name: 'awesome-react-native', description: 'Awesome React Native components, news, tools, and learning material', stars: '36k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'Solido', name: 'awesome-flutter', description: 'An awesome list of the best Flutter libraries, tools, and tutorials', stars: '59k', language: 'Markdown', tags: ['reference', 'curated'] },
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
			{ owner: 'eza-community', name: 'eza', description: 'A modern alternative to ls (maintained fork of exa)', stars: '14k', language: 'Rust', tags: ['ls', 'modern'] },
			{ owner: 'jqlang', name: 'jq', description: 'Command-line JSON processor', stars: '31k', language: 'C', tags: ['json', 'parser'] },
			{ owner: 'alebcay', name: 'awesome-shell', description: 'Curated list of awesome command-line frameworks and toolkits', stars: '36k', language: 'Markdown', tags: ['reference', 'curated'] },
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
			{ owner: 'calcom', name: 'cal.com', description: 'Open source scheduling infrastructure, Calendly alternative', stars: '34k', language: 'TypeScript', tags: ['scheduling', 'calendar'] },
			{ owner: 'NocoDB', name: 'nocodb', description: 'Open-source Airtable alternative for no-code databases', stars: '51k', language: 'TypeScript', tags: ['no-code', 'airtable'] },
			{ owner: 'makeplane', name: 'plane', description: 'Open-source project management, Jira and Linear alternative', stars: '32k', language: 'TypeScript', tags: ['project-management', 'kanban'] },
			{ owner: 'twentyhq', name: 'twenty', description: 'Open-source CRM, modern alternative to Salesforce', stars: '25k', language: 'TypeScript', tags: ['crm', 'sales'] },
			{ owner: 'RocketChat', name: 'Rocket.Chat', description: 'Open-source team communication platform, Slack alternative', stars: '41k', language: 'TypeScript', tags: ['chat', 'messaging'] },
			{ owner: 'chatwoot', name: 'chatwoot', description: 'Open-source customer engagement platform, Intercom alternative', stars: '21k', language: 'Ruby', tags: ['support', 'helpdesk'] },
			{ owner: 'appwrite', name: 'appwrite', description: 'Secure backend server for web and mobile developers', stars: '47k', language: 'TypeScript', tags: ['baas', 'backend'] },
			{ owner: 'mattermost', name: 'mattermost', description: 'Open source platform for secure collaboration', stars: '31k', language: 'TypeScript', tags: ['chat', 'collaboration'] },
			{ owner: 'plausible', name: 'analytics', description: 'Simple, privacy-friendly alternative to Google Analytics', stars: '21k', language: 'Elixir', tags: ['analytics', 'privacy'] },
			{ owner: 'posthog', name: 'posthog', description: 'Open-source product analytics, alternative to Mixpanel', stars: '24k', language: 'Python', tags: ['analytics', 'product'] },
			{ owner: 'serhii-londar', name: 'open-source-mac-os-apps', description: 'Awesome list of open source applications for macOS', stars: '47k', language: 'Markdown', tags: ['macos', 'curated'] },
		]
	},
	{
		id: 'automation',
		label: 'Automation & Workflow',
		icon: '>>',
		repos: [
			{ owner: 'n8n-io', name: 'n8n', description: 'Free and source-available workflow automation tool, Zapier alternative', stars: '52k', language: 'TypeScript', tags: ['workflow', 'low-code'] },
			{ owner: 'activepieces', name: 'activepieces', description: 'Open-source no-code workflow automation, Zapier alternative', stars: '11k', language: 'TypeScript', tags: ['automation', 'no-code'] },
			{ owner: 'temporalio', name: 'temporal', description: 'Durable execution platform for microservice orchestration', stars: '13k', language: 'Go', tags: ['orchestration', 'microservices'] },
			{ owner: 'windmill-labs', name: 'windmill', description: 'Turn scripts into webhooks, workflows and UIs', stars: '11k', language: 'Rust', tags: ['scripts', 'platform'] },
			{ owner: 'PrefectHQ', name: 'prefect', description: 'Modern workflow orchestration for data pipelines', stars: '17k', language: 'Python', tags: ['data', 'orchestration'] },
			{ owner: 'ToolJet', name: 'ToolJet', description: 'Open-source low-code platform for internal tools, Retool alternative', stars: '34k', language: 'TypeScript', tags: ['low-code', 'internal-tools'] },
			{ owner: 'Budibase', name: 'budibase', description: 'Open-source low-code platform for building internal apps', stars: '23k', language: 'TypeScript', tags: ['low-code', 'internal-tools'] },
			{ owner: 'formbricks', name: 'formbricks', description: 'Open-source experience management, privacy-first surveys', stars: '9k', language: 'TypeScript', tags: ['surveys', 'feedback'] },
			{ owner: 'flagsmith', name: 'flagsmith', description: 'Open-source feature flag and remote config service', stars: '5k', language: 'Python', tags: ['feature-flags', 'config'] },
		]
	},
	{
		id: 'data-engineering',
		label: 'Data Engineering',
		icon: '||',
		repos: [
			{ owner: 'pandas-dev', name: 'pandas', description: 'Flexible and powerful data analysis and manipulation library for Python', stars: '44k', language: 'Python', tags: ['dataframes', 'analysis'] },
			{ owner: 'apache', name: 'spark', description: 'Unified analytics engine for large-scale data processing', stars: '40k', language: 'Scala', tags: ['big-data', 'distributed'] },
			{ owner: 'apache', name: 'superset', description: 'Apache Superset is a data visualization and exploration platform', stars: '64k', language: 'TypeScript', tags: ['visualization', 'dashboards'] },
			{ owner: 'apache', name: 'airflow', description: 'Platform to programmatically author, schedule and monitor workflows', stars: '38k', language: 'Python', tags: ['workflow', 'orchestration'] },
			{ owner: 'plotly', name: 'plotly.js', description: 'Open-source JavaScript charting library', stars: '18k', language: 'JavaScript', tags: ['charts', 'visualization'] },
			{ owner: 'jupyter', name: 'notebook', description: 'Jupyter Interactive Notebook', stars: '12k', language: 'Python', tags: ['notebook', 'interactive'] },
			{ owner: 'metabase', name: 'metabase', description: 'Easy-to-use open source BI and analytics tool', stars: '40k', language: 'Clojure', tags: ['bi', 'analytics'] },
			{ owner: 'dbt-labs', name: 'dbt-core', description: 'Transform data in your warehouse using SQL and software engineering practices', stars: '10k', language: 'Python', tags: ['sql', 'transformation'] },
			{ owner: 'airbytehq', name: 'airbyte', description: 'Open-source data integration platform for ELT pipelines', stars: '16k', language: 'Python', tags: ['etl', 'integration'] },
			{ owner: 'dagster-io', name: 'dagster', description: 'Orchestration platform for data assets', stars: '12k', language: 'Python', tags: ['orchestration', 'data'] },
			{ owner: 'meltano', name: 'meltano', description: 'Open-source platform for the whole data lifecycle', stars: '2k', language: 'Python', tags: ['etl', 'singer'] },
			{ owner: 'redpanda-data', name: 'redpanda', description: 'Streaming data platform, Kafka API compatible at 10x performance', stars: '10k', language: 'C++', tags: ['streaming', 'kafka'] },
			{ owner: 'trinodb', name: 'trino', description: 'Fast distributed SQL query engine for big data analytics', stars: '10k', language: 'Java', tags: ['sql', 'big-data'] },
			{ owner: 'StarRocks', name: 'starrocks', description: 'Next-gen sub-second MPP OLAP database', stars: '9k', language: 'C++', tags: ['olap', 'analytics'] },
			{ owner: 'cube-js', name: 'cube', description: 'Headless BI platform for building data applications', stars: '18k', language: 'TypeScript', tags: ['analytics', 'semantic-layer'] },
		]
	},
	{
		id: 'communication',
		label: 'Communication',
		icon: '<>',
		repos: [
			{ owner: 'novuhq', name: 'novu', description: 'Open-source notification infrastructure for developers', stars: '35k', language: 'TypeScript', tags: ['notifications', 'email'] },
			{ owner: 'mattermost', name: 'focalboard', description: 'Open-source alternative to Trello, Notion, and Asana', stars: '22k', language: 'TypeScript', tags: ['kanban', 'project-management'] },
			{ owner: 'irccloud', name: 'ios', description: 'IRCCloud iOS App', stars: '3k', language: 'Objective-C', tags: ['irc', 'chat'] },
			{ owner: 'unsplash', name: 'unsplash-js', description: 'Official JavaScript wrapper for the Unsplash API', stars: '2k', language: 'TypeScript', tags: ['photos', 'api'] },
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
	{
		id: 'languages',
		label: 'Languages & Runtimes',
		icon: '{}',
		repos: [
			{ owner: 'JetBrains', name: 'kotlin', description: 'The Kotlin Programming Language', stars: '50k', language: 'Kotlin', tags: ['jvm', 'language'] },
			{ owner: 'arduino', name: 'Arduino', description: 'Arduino IDE for learning electronics and programming', stars: '14k', language: 'Java', tags: ['embedded', 'ide'] },
			{ owner: 'avelino', name: 'awesome-go', description: 'A curated list of awesome Go frameworks, libraries and software', stars: '165k', language: 'Go', tags: ['reference', 'curated'] },
			{ owner: 'vinta', name: 'awesome-python', description: 'An opinionated list of awesome Python frameworks, libraries and resources', stars: '230k', language: 'Python', tags: ['reference', 'curated'] },
			{ owner: 'enaqx', name: 'awesome-react', description: 'A collection of awesome things regarding the React ecosystem', stars: '72k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'sindresorhus', name: 'awesome-nodejs', description: 'Delightful Node.js packages and resources', stars: '65k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'fffaraz', name: 'awesome-cpp', description: 'A curated list of awesome C++ frameworks, libraries and resources', stars: '70k', language: 'Markdown', tags: ['reference', 'curated'] },
			{ owner: 'akullpp', name: 'awesome-java', description: 'A curated list of awesome Java frameworks and software', stars: '47k', language: 'Markdown', tags: ['reference', 'curated'] },
		]
	},
	{
		id: 'curated-lists',
		label: 'Curated Lists',
		icon: '[]',
		repos: [
			{ owner: 'sindresorhus', name: 'awesome', description: 'Awesome lists about all kinds of interesting topics', stars: '439k', language: 'Markdown', tags: ['meta', 'reference'] },
			{ owner: 'jaywcjlove', name: 'awesome-mac', description: 'Collect premium software in various categories for macOS', stars: '99k', language: 'Markdown', tags: ['macos', 'software'] },
			{ owner: 'tiimgreen', name: 'github-cheat-sheet', description: 'Cool features of Git and GitHub you might not know', stars: '55k', language: 'Markdown', tags: ['git', 'reference'] },
			{ owner: 'veggiemonk', name: 'awesome-docker', description: 'A curated list of Docker resources and projects', stars: '35k', language: 'Markdown', tags: ['docker', 'reference'] },
			{ owner: 'kuchin', name: 'awesome-cto', description: 'Resources for Chief Technology Officers, with emphasis on startups', stars: '34k', language: 'Markdown', tags: ['leadership', 'reference'] },
			{ owner: 'awesome-foss', name: 'awesome-sysadmin', description: 'Curated list of amazingly awesome open-source sysadmin resources', stars: '33k', language: 'Markdown', tags: ['sysadmin', 'reference'] },
			{ owner: 'herrbischoff', name: 'awesome-macos-command-line', description: 'Use your macOS terminal shell to do awesome things', stars: '30k', language: 'Markdown', tags: ['macos', 'cli'] },
			{ owner: 'academic', name: 'awesome-datascience', description: 'An awesome Data Science repository to learn and apply', stars: '28k', language: 'Markdown', tags: ['data-science', 'reference'] },
			{ owner: 'lukasz-madon', name: 'awesome-remote-job', description: 'A curated list of awesome remote jobs and resources', stars: '44k', language: 'Markdown', tags: ['remote', 'career'] },
			{ owner: 'PatrickJS', name: 'awesome-cursorrules', description: 'Configuration files that enhance Cursor AI editor experience', stars: '38k', language: 'Markdown', tags: ['ai', 'editor'] },
			{ owner: 'wong2', name: 'awesome-mcp-servers', description: 'A curated list of Model Context Protocol servers', stars: '15k', language: 'Markdown', tags: ['mcp', 'ai'] },
		]
	},
];

export const allLanguages = [...new Set(categories.flatMap(c => c.repos.map(r => r.language)))].sort();
export const allTags = [...new Set(categories.flatMap(c => c.repos.flatMap(r => r.tags)))].sort();
