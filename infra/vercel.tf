resource "vercel_project" "main" {
  name           = "gitarena"
  framework      = "nextjs"
  root_directory = "gitarena-frontend"

  git_repository = {
    type = "github"
    repo = "mellowagain/gitarena"
  }
}

resource "vercel_project_domain" "frontend" {
  project_id = vercel_project.main.id
  domain     = var.frontend_domain
}

data "vercel_domain_config" "frontend" {
  domain             = var.frontend_domain
  project_id_or_name = vercel_project.main.id
}

resource "vercel_project_environment_variable" "next_public_api_url" {
  project_id = vercel_project.main.id

  key       = "NEXT_PUBLIC_API_URL"
  value     = "https://${var.backend_domain}"
  sensitive = false

  target = ["production", "preview", "development"]
}
