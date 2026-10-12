//! Middleware CORS global du service.

// Request et State permettent de construire les middlewares HTTP communs.
use axum::extract::Request;
// Les types HTTP servent aux headers CORS et aux reponses preflight.
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
// Next fait avancer la requete dans la pile Axum.
use axum::middleware::Next;
// IntoResponse convertit les preflights CORS en reponses Axum.
use axum::response::{IntoResponse, Response};

/// Ajoute les en-tetes CORS necessaires au client Expo Web.
pub(super) async fn add_cors_headers(request: Request, next: Next) -> Response {
    // Les requetes preflight `OPTIONS` doivent repondre sans passer par les handlers metier.
    if request.method() == Method::OPTIONS {
        // `StatusCode::NO_CONTENT` produit une reponse vide adaptee au preflight.
        let mut response = StatusCode::NO_CONTENT.into_response();
        // Les headers CORS sont appliques au meme endroit que pour les reponses normales.
        apply_cors_headers(response.headers_mut());
        // Le preflight s'arrete ici pour eviter de traverser les middlewares metier.
        return response;
    }

    // Pour les requetes normales, Axum produit d'abord la reponse metier.
    let mut response = next.run(request).await;
    // Les headers CORS sont ajoutes ensuite de facon uniforme.
    apply_cors_headers(response.headers_mut());
    // La reponse enrichie retourne au client.
    response
}

/// Centralise les headers CORS pour eviter des divergences entre GET/PATCH et preflight.
fn apply_cors_headers(headers: &mut HeaderMap) {
    // Le client web n'utilise pas de cookie d'auth backend, donc `*` reste compatible avec le contrat actuel.
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    // Les methodes restent limitees aux endpoints HTTP exposes par l'API.
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET,POST,PATCH,DELETE,OPTIONS"),
    );
    // `authorization` permet au client de transmettre le JWT Better Auth.
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization,content-type,if-match,x-local-session"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cors_allows_protected_api_mutation_methods() {
        let mut headers = HeaderMap::new();

        apply_cors_headers(&mut headers);

        assert_eq!(
            headers
                .get(header::ACCESS_CONTROL_ALLOW_METHODS)
                .and_then(|value| value.to_str().ok()),
            Some("GET,POST,PATCH,DELETE,OPTIONS")
        );
        assert_eq!(
            headers
                .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
                .and_then(|value| value.to_str().ok()),
            Some("authorization,content-type,if-match,x-local-session")
        );
    }
}
