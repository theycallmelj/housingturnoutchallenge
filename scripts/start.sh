#!/usr/bin/env bash
# Start the State Housing Policy Lab web UI at http://127.0.0.1:8787
# (the port can be changed with HOUSING_WEB_PORT).
#
# With OPENROUTER_API_KEY set (as a Codespaces secret, an environment
# variable, or in agents/.env), the expert panel runs on live Apache-2.0
# models on OpenRouter: gpt-oss-120b leads, gpt-oss-20b experts (the
# "openrouter" profile in agents/model-registry.json). Without a key it runs
# the offline deterministic models; the scenario model, baseline and
# back-tests work the same either way.
#
#     scripts/start.sh
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -f agents/.env ]; then
  set -a
  # shellcheck disable=SC1091
  . agents/.env
  set +a
fi

export HOUSING_WEB_PORT="${HOUSING_WEB_PORT:-8787}"
features=()
if [ -n "${OPENROUTER_API_KEY:-}" ]; then
  export HOUSING_LLM_HOST="${HOUSING_LLM_HOST:-openrouter}"
  export HOUSING_MODEL_PROFILE="${HOUSING_MODEL_PROFILE:-openrouter}"
  features=(--features llm)
  echo "Expert panel: live models on OpenRouter (profile ${HOUSING_MODEL_PROFILE})."
elif [ -n "${HOUSING_LLM_BASE_URL:-}" ]; then
  features=(--features llm)
  echo "Expert panel: models at ${HOUSING_LLM_BASE_URL} (${HOUSING_LLM_HOST:-ollama})."
else
  echo "Expert panel: offline models (no OPENROUTER_API_KEY set)."
fi
echo "Open http://127.0.0.1:${HOUSING_WEB_PORT} (in Codespaces: the Ports tab, port ${HOUSING_WEB_PORT})."
exec cargo run --quiet --release "${features[@]}" --manifest-path agents/Cargo.toml --bin housing-web
