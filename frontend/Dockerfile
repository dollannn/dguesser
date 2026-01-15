# Build stage
FROM oven/bun:1 AS builder

WORKDIR /app

# Copy package files
COPY package.json bun.lock ./

# Install dependencies
RUN bun install --frozen-lockfile

# Copy source files
COPY . .

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

# Copy built application
COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules

EXPOSE 3000

CMD ["build"]
