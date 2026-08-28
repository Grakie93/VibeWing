# Contributing to VibeWing

Thanks for helping VibeWing make local project operations easier for vibe coders.

## Before contributing

- Search existing issues before opening a new one.
- Use sanitized examples; never attach credentials, private repository URLs, proprietary source, or raw logs containing tokens.
- Discuss large features in an issue before implementation.
- Keep changes cross-platform unless an issue is explicitly platform-specific.
- Preserve existing user data and avoid destructive Git or filesystem behavior.

## Development setup

```bash
git clone https://github.com/Grakie93/VibeWing.git
cd VibeWing
npm ci
python3 -m pip install -r requirements-build.txt
npm start
```

## Pull requests

- Explain the user problem and the chosen behavior.
- Include macOS and/or Windows test results as applicable.
- Update user-facing English and Chinese text together.
- Update README or release documentation when behavior changes.
- Do not commit local user data, build output, logs, API keys, signing material, or `.env` files.

## Licensing

VibeWing is licensed under MIT. By contributing, you agree that your contribution is provided under the repository's MIT License and certify that you have the right to submit it. The project may adopt a Developer Certificate of Origin sign-off if contribution volume grows; a heavy CLA is not required for the current MIT community model.
