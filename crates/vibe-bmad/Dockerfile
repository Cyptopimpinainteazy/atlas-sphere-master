FROM node:20

WORKDIR /workspace

COPY . /workspace

RUN npm ci --silent || true

ENTRYPOINT ["/bin/sh", "-c"]
CMD ["npx bmad-method@alpha --version || true && npx bmad-method@alpha install && npx bmad-method status || true"]
