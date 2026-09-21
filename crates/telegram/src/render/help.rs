/// Commands shown by `/ajuda` and registered in the bot menu.
pub const COMMANDS: &[(&str, &str)] = &[
    ("gasto", "Registrar um gasto"),
    ("entrada", "Registrar uma entrada (salário, extra...)"),
    ("transferir", "Mover dinheiro entre contas"),
    ("guardar", "Guardar dinheiro numa meta"),
    ("resgatar", "Tirar dinheiro de uma meta"),
    ("pagarfatura", "Pagar a fatura de um cartão"),
    ("estorno", "Registrar um estorno ou reembolso"),
    ("fatura", "Faturas dos cartões"),
    ("saldo", "Saldo das contas e disponível"),
    ("resumo", "Resumo de hoje; /resumo 15/09 para outro dia"),
    ("ontem", "Resumo de ontem"),
    ("metas", "Progresso das metas"),
    ("mes", "Como está o ciclo até agora"),
    ("orcamento", "Definir ou remover o limite de uma categoria"),
    ("orcamentos", "Orçamentos do ciclo"),
    ("desfazer", "Desfazer seu último lançamento"),
    ("ultimos", "Últimos lançamentos, com editar e apagar"),
    ("exportar", "Planilha (CSV) do ciclo; /exportar 03/2026 para outro"),
    ("novaconta", "Cadastrar uma conta"),
    ("novameta", "Criar uma meta de economia"),
    ("novocartao", "Cadastrar um cartão de crédito"),
    ("cartoes", "Listar cartões"),
    ("recorrente", "Criar um lançamento recorrente (salário, aluguel...)"),
    ("recorrentes", "Listar e desativar recorrentes"),
    ("contas", "Listar contas"),
    ("categorias", "Listar categorias"),
    ("ajuste", "Acertar o saldo de uma conta com o do banco"),
    ("config", "Dia de início do ciclo e horários dos resumos"),
    ("cancelar", "Cancelar o que está preenchendo"),
    ("ajuda", "Mostrar os comandos"),
];

pub fn help_text() -> String {
    let lines: Vec<String> = COMMANDS
        .iter()
        .map(|(command, description)| format!("/{command} — {description}"))
        .collect();
    format!("<b>Comandos</b>\n{}", lines.join("\n"))
}

pub fn welcome_text() -> String {
    format!("Olá! 👋 Vou registrar as finanças de vocês aqui.\n\n{}", help_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_lists_every_command() {
        let text = help_text();
        assert!(COMMANDS.iter().all(|(command, _)| text.contains(&format!("/{command} "))));
        assert!(welcome_text().starts_with("Olá!"));
    }
}
