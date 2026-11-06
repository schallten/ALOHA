FROM python:3.13-slim

WORKDIR /app

COPY pyproject.toml ./
RUN pip install --no-cache-dir "fluxer-py @ git+https://github.com/Fluxer-py/fluxer.py@rework" "python-dotenv>=1.0"

COPY main.py constants.py pacer.py sanitize.py state.py ./

ENV PYTHONUNBUFFERED=1

CMD ["python", "main.py"]
