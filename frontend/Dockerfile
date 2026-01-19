# Build stage
FROM oven/bun:1 AS builder

WORKDIR /app

# Copy package files
COPY package.json bun.lock ./

# Install dependencies
RUN bun install --frozen-lockfile

# Copy source files
COPY . .

# Build-time arguments for Vite (baked into JS bundle)
ARG VITE_API_URL
ARG VITE_REALTIME_URL
ARG VITE_GOOGLE_MAPS_API_KEY

# Set as environment variables for the build process
ENV VITE_API_URL=$VITE_API_URL
ENV VITE_REALTIME_URL=$VITE_REALTIME_URL
ENV VITE_GOOGLE_MAPS_API_KEY=$VITE_GOOGLE_MAPS_API_KEY

# Build the application
RUN bun run build

# Runtime stage
FROM gcr.io/distroless/nodejs22-debian12

WORKDIR /app

# Set production environment
ENV NODE_ENV=production
ENV HOST=0.0.0.0

# Railway sets PORT automatically, default to 3000
ENV PORT=3000

# Runtime environment variables for SSR (set via Railway env vars)
# API_URL - Server-side API URL for SSR requests (e.g., http://api-service:3001 or https://api.dguesser.lol)

# Copy built application
COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules

EXPOSE 3000

CMD ["build"]
