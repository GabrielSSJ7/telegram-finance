//! Use-case errors as chat replies (pt-BR). Storage details stay in logs.

use app::AppError;

use crate::html::escape;

pub fn error_text(error: &AppError) -> String {
    match error {
        AppError::NotFound { .. } => "Não encontrei esse registro. Pode ter sido apagado.".into(),
        AppError::Invalid { field, value, .. } => {
            format!("Não deu certo: {} inválido ({}).", field_name(field), escape(value))
        }
        AppError::Conflict(_) => "Já existe um registro com esse nome.".into(),
        AppError::AlreadyCommitted => "Isso já foi registrado ✅".into(),
        AppError::Unauthorized | AppError::Forbidden(_) => {
            "Você não tem acesso a este bot. 🔒".into()
        }
        AppError::Storage(_) => {
            "Estou com problema para acessar os dados. Tente de novo em instantes.".into()
        }
    }
}

fn field_name(field: &str) -> &str {
    match field {
        "amount" => "valor",
        "description" => "descrição",
        "category" => "categoria",
        "destination account" => "conta de destino",
        "account kind" => "tipo de conta",
        "account name" | "goal name" => "nome",
        "goal target" => "objetivo",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_each_error_kind() {
        let invalid = AppError::invalid("amount", "<5>", "at most 10");
        assert_eq!(error_text(&invalid), "Não deu certo: valor inválido (&lt;5&gt;).");
        assert!(error_text(&AppError::not_found("entry", 1)).contains("Não encontrei"));
        assert!(error_text(&AppError::Conflict("x".into())).contains("Já existe"));
        assert!(error_text(&AppError::AlreadyCommitted).contains("já foi registrado"));
        assert!(error_text(&AppError::Forbidden(1)).contains("acesso"));
        assert!(!error_text(&AppError::Storage("password=x".into())).contains("password"));
        assert!(error_text(&AppError::invalid("weird", 1, "x")).contains("weird"));
    }
}
