# AI-Brain Architecture & Implementation Plan

*(XiaoAi Device ⇄ AI-Brain ⇄ LLM Backend)*

------------------------------------------------------------------------

# 0. System Overview

## Goal

Build an intermediate **AI-Brain service** that:

-   Accepts WebSocket streaming text from XiaoAi device
-   Sends requests to LLM backend
-   Streams responses back via WebSocket
-   Supports interruption
-   Can switch between local and cloud LLM providers

------------------------------------------------------------------------

# Phase 1 --- Minimal Working Local Setup (LM Studio)

## Objective

Make it work simply and reliably using a local LM Studio server.

## Architecture

XiaoAi\
⇅ WebSocket\
AI-Brain (Node / Python server)\
⇅ HTTP (OpenAI-compatible API)\
LM Studio (local LLM)

## Responsibilities

### XiaoAi Device

-   Open persistent WebSocket connection
-   Send:

``` json
{
  "type": "user_input",
  "session_id": "...",
  "text": "Hello world"
}
```

-   Receive streaming tokens:

``` json
{
  "type": "llm_token",
  "token": "Hello"
}
```

### AI-Brain Service

-   Maintain session memory per connection
-   Convert input to OpenAI-compatible format
-   Call LM Studio with:

``` json
{
  "stream": true
}
```

-   Parse SSE streaming response
-   Forward incremental tokens over WebSocket

### LM Studio

-   Runs local model
-   Exposes OpenAI-compatible HTTP endpoint

## Key Design Principles (Phase 1)

-   Stateless WebSocket protocol
-   Per-session conversation memory inside AI-Brain
-   Abort generation if WebSocket disconnects
-   No interruption yet

------------------------------------------------------------------------

# Phase 2 --- Streaming + Controlled Interruption

## Objective

Add ability to interrupt generation when:

-   User sends new input
-   Specific trigger word detected
-   Device sends explicit "interrupt" command

## Interruption Model (HTTP Streaming)

1.  AI-Brain starts streaming LLM response.
2.  If:
    -   New user input arrives
    -   Or keyword detected (e.g., "stop", "cancel")
3.  AI-Brain:
    -   Cancels HTTP request (close connection)
    -   Stops forwarding tokens
    -   Starts new generation

## WebSocket Protocol Extension

``` json
{
  "type": "interrupt",
  "session_id": "..."
}
```

## State Machine (Per Session)

States:

-   IDLE
-   GENERATING
-   INTERRUPTED

Transitions:

-   IDLE → GENERATING
-   GENERATING → INTERRUPTED
-   INTERRUPTED → GENERATING
-   GENERATING → IDLE

------------------------------------------------------------------------

# Phase 3 --- Streaming Input (Incremental User Text)

## Input Streaming Model

``` json
{
  "type": "user_partial",
  "text": "What is the wea"
}
```

``` json
{
  "type": "user_partial",
  "text": "What is the weather in"
}
```

``` json
{
  "type": "user_final",
  "text": "What is the weather in Tokyo?"
}
```

## Strategy Options

### Option A (Simple)

-   Only trigger LLM on `user_final`

### Option B (Low Latency)

-   Start generation on partial
-   Restart if partial changes significantly

------------------------------------------------------------------------

# Phase 4 --- Voice-Ready Architecture

Speech-to-Text → Text Stream → AI-Brain → Text → TTS

Requirements:

-   Token-level streaming output
-   Ability to stop TTS when interrupted
-   Fast token-to-audio conversion

------------------------------------------------------------------------

# Phase 5 --- Cloud LLM Integration

## Abstraction Layer

``` python
class LLMProvider:
    async def generate_stream(messages): ...
    async def cancel(): ...
```

Implementations:

-   LocalProvider
-   CloudProvider
-   MultiProviderRouter

------------------------------------------------------------------------

# WebSocket Protocol Design

## Client → AI-Brain

-   connect
-   user_partial
-   user_final
-   interrupt
-   ping

## AI-Brain → Client

-   ack
-   llm_token
-   llm_end
-   interrupted
-   error

------------------------------------------------------------------------

# Deployment Strategy

## Stage 1

-   Local LM Studio
-   Single-session test

## Stage 2

-   Add interruption logic
-   Multi-session support

## Stage 3

-   Containerize AI-Brain
-   Deploy to cloud VM

## Stage 4

-   Add authentication
-   Add rate limiting
-   Add monitoring

------------------------------------------------------------------------

# Final Architecture Vision

XiaoAi Device\
⇅ WebSocket\
AI-Brain Gateway\
⇅ Provider abstraction\
⇅ Local or Cloud LLM

Responsibilities:

-   Session management
-   Streaming transformation
-   Interruption control
-   Provider abstraction
-   Routing logic
-   Security boundary

