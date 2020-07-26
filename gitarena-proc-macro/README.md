# gitarena-proc-macro

Rust does not allow for proc macros to be declared and used within the same crate.
Thereby a secondary crate was created to hold these. The following macros are available:

## `generate_bail!(Structure)`

Generates a `bail!` macro to be used within routes. GitArena handles errors normally
using `anyhow` - with the exception of routes. Within routes, any method that returns
`Result` and doesn't require special handling is wrapped within the `bail!` macro
which will return a JsonResponse with the provided structure indicating an internal
server occurred while handling the request.
