"""
MkDocs macros module — exposes selected environment variables to templates,
falling back to public GitHub URLs when the env var is unset or empty.

The fallbacks let `mkdocs build` produce a sensible site even when
`docker/.env.mkdocs` is absent (e.g., on CI or for the published site).
A local `docker/.env.mkdocs` overrides them at compose time.
"""
import os


DEFAULTS = {
    "FLATROOT_REPO_URL":    "https://github.com/flatroot/flatroot",
    "FLATROOT_RELEASE_URL": "https://github.com/flatroot/flatroot/releases/latest/download",
}


def define_env(env):
    for var, default in DEFAULTS.items():
        env.variables[var] = os.environ.get(var) or default
