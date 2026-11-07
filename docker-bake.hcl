variable "VERGEN_GIT_SHA" {
  default = "unknown"
}

variable "CACHE_SCOPE" {
  default = ""
}

variable "PUSH_BY_DIGEST" {
  default = false
}

function "cache_from" {
  params = [name]
  result = CACHE_SCOPE == "" ? [] : ["type=gha,scope=${CACHE_SCOPE}-${name}"]
}

function "cache_to" {
  params = [name]
  result = CACHE_SCOPE == "" ? [] : ["type=gha,scope=${CACHE_SCOPE}-${name},mode=max,ignore-error=true"]
}

function "local_tags" {
  params = [name]
  result = PUSH_BY_DIGEST ? [] : ["gitarena/${name}:local"]
}

function "output" {
  params = [image]
  result = PUSH_BY_DIGEST ? ["type=image,\"name=ghcr.io/mellowagain/${image},docker.io/mellowagain/${image}\",push-by-digest=true,name-canonical=true,push=true"] : []
}

group "default" {
  targets = ["backend", "zoekt-webserver", "frontend", "all-in-one"]
}

target "backend" {
  context    = "."
  dockerfile = "Dockerfile"
  args = {
    VERGEN_GIT_SHA = VERGEN_GIT_SHA
  }
  tags       = local_tags("backend")
  cache-from = cache_from("backend")
  cache-to   = cache_to("backend")
  output     = output("gitarena")
}

target "zoekt-webserver" {
  context    = "."
  dockerfile = "Dockerfile"
  target     = "zoekt-webserver"
  tags       = local_tags("zoekt-webserver")
  cache-from = cache_from("zoekt-webserver")
  cache-to   = cache_to("zoekt-webserver")
  output     = output("gitarena-zoekt-webserver")
}

target "frontend" {
  context    = "gitarena-frontend"
  dockerfile = "Dockerfile"
  tags       = local_tags("frontend")
  cache-from = cache_from("frontend")
  cache-to   = cache_to("frontend")
  output     = output("gitarena-frontend")
}

target "all-in-one" {
  context    = "deploy/all-in-one"
  dockerfile = "Dockerfile"
  contexts = {
    backend  = "target:backend"
    frontend = "target:frontend"
    caddy    = "deploy/compose/caddy"
    setup    = "deploy/compose/setup"
  }
  tags       = local_tags("all-in-one")
  cache-from = cache_from("all-in-one")
  cache-to   = cache_to("all-in-one")
  output     = output("gitarena-all-in-one")
}
