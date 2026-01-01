# Vibe-BMAD Integration

This wrapper enables running the BMAD Method (`bmad-method`) inside the Atlas Sphere workspace.

Requirements
- Node.js >= 20 (BMAD requires Node >= 20)

Quick start
1. Install BMAD (alpha release recommended for new features):

You can install BMAD in one of two ways:

Host install (requires Node >= 20 installed):

```sh
cd crates/vibe-bmad
npm run install-bmad
```

Docker-based install (no Node required on host):

```sh
cd crates/vibe-bmad
docker build -t vibe-bmad .
docker run --rm -ti -v "${PWD}":/workspace vibe-bmad
# Or use docker-compose
docker compose up --build bmad
```
2. Initialize the workspace analysis and recommended workflow:
npm run workflow-init
```

Makefile (convenience):

```sh
# Install using host Node if available, otherwise build and run Docker image
make install

# Run initialization (status, which checks BMAD installation) using host Node if available, otherwise Docker
make init
```

```sh
npm run status
```

If using Docker:

```sh
docker run --rm -ti -v "${PWD}":/workspace vibe-bmad npx bmad-method status
```

If you want the stable release instead, run:
```
npm run install-bmad-stable
```

Notes
- This is a small integration wrapper that calls `bmad-method` using `npx`. No code is vendored or included.
- The `bmad-method` project is an NPM package. See https://github.com/bmad-code-org/BMAD-METHOD for more details.

Troubleshooting
- Ensure `node` is available and the version is 20 or greater:
```
node --version
```

# License
MIT
