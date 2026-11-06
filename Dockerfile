FROM python:3.13-slim AS builder

# uv — copied from the official Astral image, not pip-installed
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# UV_HTTP_TIMEOUT / retries: helps on slow networks (e.g. Azure -> pypi.org).
ENV UV_HTTP_TIMEOUT=120 UV_HTTP_RETRIES=10 UV_LINK_MODE=copy VIRTUAL_ENV=/opt/venv PATH="/opt/venv/bin:$PATH"

WORKDIR /app
RUN uv venv /opt/venv --python /usr/local/bin/python

# fluxer-py's pyproject.toml requires setuptools>=68 to build. Pre-install
# it so uv can build fluxer-py with --no-build-isolation instead of
# re-downloading setuptools inside an isolated build env (which timed out
# on Azure -> pypi.org with pip).
COPY pyproject.toml ./
RUN uv pip install --no-cache "setuptools>=68" wheel \
    && uv pip install --no-cache --no-build-isolation \
        "fluxer-py @ git+https://github.com/Fluxer-py/fluxer.py@rework" "python-dotenv>=1.0"

FROM python:3.13-slim

ENV PYTHONUNBUFFERED=1 VIRTUAL_ENV=/opt/venv PATH="/opt/venv/bin:$PATH"

WORKDIR /app

COPY --from=builder /opt/venv /opt/venv
COPY main.py constants.py pacer.py sanitize.py state.py ./

CMD ["python", "main.py"]
