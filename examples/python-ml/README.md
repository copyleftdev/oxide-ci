# Python ML Pipeline Example

End-to-end ML pipeline with training, evaluation, and SageMaker deployment.

## Features Demonstrated

- **Nix environment** — Reproducible Python with CUDA
- **Firecracker** — Isolated training with high resources
- **Large artifacts** — Model storage with retention
- **Approval gates** — Human review before production
- **Scheduled runs** — Weekly retraining via cron

## Pipeline Flow

```
setup (venv, deps)
       │
       ▼
validate (lint, test)
       │
       ▼
train (Firecracker, 8 vCPU, 32GB)
       │
       ▼
evaluate (metrics, reports)
       │
       ▼
deploy (approval → SageMaker)
```

## Nix Flake

```nix
# flake.nix
{
  outputs = { nixpkgs, ... }:
    let pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in {
      devShells.x86_64-linux.ml-devshell = pkgs.mkShell {
        packages = with pkgs; [
          python311
          python311Packages.pip
          python311Packages.virtualenv
        ];
      };
    };
}
```

## Run Locally

```bash
oxide-ci run --stage validate  # Quick lint/test
oxide-ci run --stage train     # Full training (needs resources)
```
