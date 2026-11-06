FROM python:3.13-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
RUN python -m venv /opt/venv
ENV VIRTUAL_ENV=/opt/venv
ENV PATH="/opt/venv/bin:$PATH"

COPY pyproject.toml ./
RUN pip install --no-cache-dir "fluxer-py @ git+https://github.com/Fluxer-py/fluxer.py@rework" "python-dotenv>=1.0"

FROM python:3.13-slim

ENV PYTHONUNBUFFERED=1
ENV VIRTUAL_ENV=/opt/venv
ENV PATH="/opt/venv/bin:$PATH"

WORKDIR /app

COPY --from=builder /opt/venv /opt/venv
COPY main.py constants.py pacer.py sanitize.py state.py ./

CMD ["python", "main.py"]
