use axum::response::{IntoResponse, Response};

/// Helper trait to convert the error of a result into a [`Response`].
pub trait ErrIntoResponse {
    type Ok;

    /// Convert the `Err` variant to [`Response`].
    #[allow(clippy::result_large_err)]
    fn err_into_response(self) -> Result<Self::Ok, Response>;
}

impl<T, E: IntoResponse> ErrIntoResponse for Result<T, E> {
    type Ok = T;

    fn err_into_response(self) -> Result<T, Response> {
        self.map_err(IntoResponse::into_response)
    }
}
