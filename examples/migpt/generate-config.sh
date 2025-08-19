#!/bin/bash

cd "$(dirname "$0")"

# Load environment variables from .env file
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Check required environment variables
required_vars=("OPENAI_BASE_URL" "OPENAI_API_KEY" "OPENAI_MODEL")
missing_vars=()

for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        missing_vars+=("$var")
    fi
done

if [ ${#missing_vars[@]} -ne 0 ]; then
    echo "❌ Missing required environment variables:"
    printf '   %s\n' "${missing_vars[@]}"
    echo ""
    echo "💡 Copy .env.example to .env and fill in your values:"
    echo "   cp .env.example .env"
    echo "   # Edit .env with your actual values"
    exit 1
fi

# Generate config.ts from template
echo "🔧 Generating config.ts from template..."
envsubst < config.template.ts > config.ts

echo "✅ config.ts generated successfully!"
echo "📄 Using:"
echo "   API Key: ${OPENAI_API_KEY:0:10}..."
echo "   Model: ${OPENAI_MODEL}"
echo "   Base URL: ${OPENAI_BASE_URL}"
