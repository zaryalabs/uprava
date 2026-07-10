FROM node:24-bookworm-slim AS builder
WORKDIR /app
ENV NODE_OPTIONS=--max-old-space-size=4096
ARG VITE_UPRAVA_API_BASE=http://127.0.0.1:8080/api/v1
ENV VITE_UPRAVA_API_BASE=${VITE_UPRAVA_API_BASE}
COPY package.json package-lock.json* ./
RUN npm install
COPY . .
RUN npm run build

FROM node:24-bookworm-slim
WORKDIR /app
ENV NODE_ENV=production \
    HOME=/home/node
COPY --from=builder /app/package.json /app/package-lock.json* ./
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/dist ./dist
RUN chown -R node:node /app
USER node
EXPOSE 5173
CMD ["npm", "run", "preview", "--", "--host", "0.0.0.0", "--port", "5173"]
