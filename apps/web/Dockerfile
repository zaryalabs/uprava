FROM node:24-bookworm-slim@sha256:cb4e8f7c443347358b7875e717c29e27bf9befc8f5a26cf18af3c3dec80e58c5 AS builder
WORKDIR /app
ENV NODE_OPTIONS=--max-old-space-size=4096
ARG VITE_UPRAVA_API_BASE=http://127.0.0.1:8080/api/v1
ARG VITE_UPRAVA_RELEASE_ID=dev
ENV VITE_UPRAVA_API_BASE=${VITE_UPRAVA_API_BASE} \
    VITE_UPRAVA_RELEASE_ID=${VITE_UPRAVA_RELEASE_ID}
COPY package.json package-lock.json* ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginxinc/nginx-unprivileged:1.29.4-alpine@sha256:a6c4f61f456b85b8fdf7ec7ab28cc3e299440e6fb4a9dea520e5fd8fd440025e
COPY --from=builder --chown=101:101 /app/dist /usr/share/nginx/html
COPY --chown=101:101 nginx.conf /etc/nginx/conf.d/default.conf
USER 101
EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
