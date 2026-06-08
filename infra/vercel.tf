resource "vercel_project" "main" {
  name           = "gitarena"
  framework      = "nextjs"
  root_directory = "gitarena-frontend"

  git_repository = {
    type = "github"
    repo = "mellowagain/gitarena"
  }
}

resource "vercel_project_environment_variable" "next_public_api_url" {
  project_id = vercel_project.main.id

  key       = "NEXT_PUBLIC_API_URL"
  value     = var.backend_domain
  sensitive = false

  target = ["production", "preview", "development"]
}
