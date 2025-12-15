# Oxide CI Pricing Model

## Philosophy

> **"Developer-first, usage-fair, enterprise-ready"**

Oxide CI follows the modern CI/CD pricing model pioneered by GitHub Actions, CircleCI, and BuildKite—charging primarily on **build minutes** with seat-based tiers for team features.

---

## Tiers

### 🆓 Open Source — Free Forever
For public repositories and open source projects.

| Feature | Limit |
|---------|-------|
| Build minutes | 2,000/month |
| Concurrent jobs | 2 |
| Agents | Community shared |
| Retention | 7 days |
| Support | Community |

---

### 🚀 Starter — $0/month
For individuals and small teams getting started.

| Feature | Limit |
|---------|-------|
| Build minutes | 500/month included |
| Overage | $0.008/minute |
| Concurrent jobs | 2 |
| Self-hosted agents | 1 |
| Secrets | 10 |
| Retention | 14 days |

---

### 💼 Professional — $15/user/month
For growing teams with production workloads.

| Feature | Limit |
|---------|-------|
| Build minutes | 3,000/month included |
| Overage | $0.006/minute |
| Concurrent jobs | 10 |
| Self-hosted agents | Unlimited |
| Secrets | Unlimited |
| OIDC auth | ✓ |
| Approval gates | ✓ |
| Notifications | ✓ |
| Retention | 30 days |
| Support | Email (48h SLA) |

---

### 🏢 Enterprise — Custom
For organizations requiring compliance, SSO, and dedicated support.

| Feature | Included |
|---------|----------|
| Build minutes | Volume pricing |
| Concurrent jobs | Unlimited |
| Self-hosted agents | Unlimited |
| SAML/OIDC SSO | ✓ |
| Audit logs | ✓ |
| Firecracker isolation | ✓ |
| On-premises deployment | ✓ |
| SLA | 99.9% uptime |
| Support | Dedicated CSM, 4h response |

---

## Usage-Based Components

| Resource | Unit | Price |
|----------|------|-------|
| Build minutes (Linux) | per minute | $0.008 |
| Build minutes (macOS) | per minute | $0.08 |
| Build minutes (Windows) | per minute | $0.016 |
| Storage | per GB/month | $0.10 |
| Data transfer | per GB | $0.05 |
| Firecracker VMs | per minute | $0.012 |

---

## Competitive Positioning

| Provider | Free Tier | Pro Price | Per-Minute |
|----------|-----------|-----------|------------|
| GitHub Actions | 2,000 min | $4/user + usage | $0.008 |
| CircleCI | 6,000 min | $15/user | $0.006 |
| BuildKite | None | $15/user | Agent-based |
| **Oxide CI** | 2,000 min | $15/user | $0.006 |

### Differentiators
- **Nix-native** — Reproducible builds without Docker
- **Firecracker isolation** — VM-level security at container speed
- **OIDC keyless auth** — No static credentials
- **Self-hosted first** — Full control, cloud optional
- **Open core** — Apache 2.0 base, enterprise features licensed

---

## Revenue Model

```
MRR = (Seats × $15) + (Overage Minutes × $0.006) + (Enterprise Contracts)
```

### Target Metrics
- **ARPU**: $50/user/month (blended)
- **Gross Margin**: 70%+ (compute costs ~30%)
- **NDR**: 120%+ (usage grows with adoption)
