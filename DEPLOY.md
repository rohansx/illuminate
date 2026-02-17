# Deploying Illuminate to VPS

Deploy Illuminate to any VPS with Docker. No Docker Hub needed — builds from source.

## Prerequisites

- VPS with Docker & Docker Compose (Ubuntu 22.04+ recommended)
- Domain pointed to your VPS IP
- GitHub OAuth app ([create here](https://github.com/settings/applications/new))
  - Callback URL: `https://yourdomain.com/auth/github/callback`

## Quick Deploy

```bash
# Clone repo
git clone https://github.com/rohansx/illuminate.git
cd illuminate

# Set environment variables
export POSTGRES_PASSWORD=$(openssl rand -hex 16)
export GITHUB_CLIENT_ID=your_github_client_id
export GITHUB_CLIENT_SECRET=your_github_client_secret
export ENCRYPT_KEY=$(openssl rand -hex 32)
export JWT_SECRET=$(openssl rand -hex 32)
export FRONTEND_URL=https://yourdomain.com
export BACKEND_URL=https://yourdomain.com
export COOKIE_DOMAIN=yourdomain.com
export ADMIN_GITHUB_USERNAME=your_github_username

# Update Caddyfile domain
sed -i 's/{$DOMAIN:illuminate.sh}/yourdomain.com/g' Caddyfile

# Deploy
docker compose up -d --build

# Check logs
docker compose logs -f app
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `POSTGRES_PASSWORD` | Database password | `openssl rand -hex 16` |
| `GITHUB_CLIENT_ID` | GitHub OAuth client ID | From GitHub OAuth app |
| `GITHUB_CLIENT_SECRET` | GitHub OAuth secret | From GitHub OAuth app |
| `ENCRYPT_KEY` | Encryption key (64 hex chars) | `openssl rand -hex 32` |
| `JWT_SECRET` | JWT signing key (64 hex chars) | `openssl rand -hex 32` |
| `FRONTEND_URL` | Frontend URL | `https://yourdomain.com` |
| `BACKEND_URL` | Backend URL | `https://yourdomain.com` |
| `COOKIE_DOMAIN` | Cookie domain | `yourdomain.com` |
| `ADMIN_GITHUB_USERNAME` | Admin GitHub username | Your username |

## Architecture

```
Internet → Caddy (ports 80/443)
  ↓ HTTPS auto-provisioned
  ↓ reverse proxy
App (Go server + SvelteKit static)
  ↓
PostgreSQL + Redis
```

## Post-Deployment

1. **Visit** `https://yourdomain.com` — Caddy auto-provisions HTTPS
2. **Login** with GitHub — Admin auto-promoted on first login
3. **Go to Admin** at `/app/admin`
4. **Seed repos** — Click "seed repos" button
5. **Index issues** — Click "index issues" button

## Admin Dashboard

Access at `/app/admin` with admin role:

- **Stats** — User/repo/issue counts
- **Seed** — Add repos from `api/data/seed_repos.json`
- **Index** — Fetch issues from all repos
- **Users** — Manage users and roles
- **Repos** — View/delete indexed repos
- **Jobs** — Monitor background tasks (live updates)

## Customizing Repositories

Edit `api/data/seed_repos.json`:

```json
[
  {"owner": "facebook", "name": "react"},
  {"owner": "microsoft", "name": "vscode"},
  {"owner": "golang", "name": "go"}
]
```

Then trigger re-seed from admin dashboard.

## Maintenance

### View Logs
```bash
docker compose logs -f app
docker compose logs -f caddy
docker compose logs -f postgres
```

### Restart Service
```bash
docker compose restart app
```

### Update Code
```bash
git pull
docker compose up -d --build app
```

### Backup Database
```bash
docker compose exec postgres pg_dump -U illuminate illuminate > backup-$(date +%Y%m%d).sql
```

### Restore Database
```bash
cat backup.sql | docker compose exec -T postgres psql -U illuminate illuminate
```

### Access Database
```bash
docker compose exec postgres psql -U illuminate
```

## Monitoring

- **Health**: `https://yourdomain.com/health`
- **App logs**: `docker compose logs -f app`
- **Stats**: Check admin dashboard

## Troubleshooting

### Port 80/443 in use
```bash
sudo lsof -i :80
sudo lsof -i :443
# Stop conflicting services
```

### OAuth callback fails
- Check GitHub OAuth app callback URL: `https://yourdomain.com/auth/github/callback`
- Verify `FRONTEND_URL` and `BACKEND_URL` match your domain
- Check `COOKIE_DOMAIN` is set correctly

### Database connection error
- Verify `POSTGRES_PASSWORD` is set
- Check: `docker compose logs postgres`

### Admin not promoted
- Verify `ADMIN_GITHUB_USERNAME` matches your exact GitHub username
- Check logs: `docker compose logs app | grep admin`

### Caddy not getting HTTPS
- Ensure port 80/443 are open in firewall
- Check DNS: `dig yourdomain.com` should return VPS IP
- Logs: `docker compose logs caddy`

## Security Checklist

- ✅ Use strong `POSTGRES_PASSWORD` (not default)
- ✅ Generate random `ENCRYPT_KEY` and `JWT_SECRET`
- ✅ Set `ILLUMINATE_ENV=production` (done in docker-compose.yml)
- ✅ Never commit `.env` files
- ✅ Keep GitHub OAuth secrets secure
- ✅ Caddy handles HTTPS automatically (Let's Encrypt)

## Performance Tuning

### Scale App Instances
```bash
docker compose up -d --scale app=3
```

### Connection Pooling
PostgreSQL default: 100 connections. For high traffic, tune in docker-compose.yml:

```yaml
postgres:
  command: postgres -c max_connections=200
```

### Use Managed Database
For production scale, use managed PostgreSQL:
- AWS RDS
- DigitalOcean Managed Database
- Supabase

Update `DATABASE_URL` in docker-compose.yml.

## Cost Estimate

| Resource | Specs | Cost/month |
|----------|-------|------------|
| VPS (DigitalOcean) | 1 vCPU, 2GB RAM | $12 |
| Domain | .com/.sh | $1 |
| **Total** | | **~$13/month** |

For 100+ concurrent users: 2 vCPU / 4GB RAM (~$24/month)

## Alternative Deployment Methods

### Without Docker

See individual READMEs:
- [Backend](api/README.md) — Go server
- [Frontend](web/README.md) — SvelteKit

### Platform as a Service

Deploy to:
- **Railway** — Connect GitHub, auto-deploy
- **Fly.io** — `fly launch` (use fly postgres)
- **Render** — Connect repo, add env vars

## Support

- **Issues**: [GitHub Issues](https://github.com/rohansx/illuminate/issues)
- **Docs**: See [README.md](README.md)
