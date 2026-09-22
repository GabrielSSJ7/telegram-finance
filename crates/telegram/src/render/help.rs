//! The command list: one source for `/ajuda` and for the menu Telegram
//! shows when someone types `/`.

/// A block of `/ajuda`, in the order the couple usually needs it.
pub struct CommandSection {
    pub title: &'static str,
    pub commands: &'static [(&'static str, &'static str)],
}

pub const SECTIONS: &[CommandSection] = &[
    CommandSection {
        title: "💸 Registrar",
        commands: &[
            ("gasto", "Registrar um gasto"),
            ("entrada", "Registrar uma entrada (salário, extra...)"),
            ("transferir", "Mover dinheiro entre contas"),
            ("guardar", "Guardar dinheiro numa meta"),
            ("resgatar", "Tirar dinheiro de uma meta"),
            ("pagarfatura", "Pagar a fatura de um cartão"),
            ("estorno", "Registrar um estorno ou reembolso"),
            ("ajuste", "Acertar o saldo de uma conta com o do banco"),
        ],
    },
    CommandSection {
        title: "👀 Ver",
        commands: &[
            ("saldo", "Saldo das contas e disponível"),
            ("resumo", "Resumo de hoje; /resumo 15/09 para outro dia"),
            ("ontem", "Resumo de ontem"),
            ("mes", "Como está o ciclo até agora"),
            ("projecao", "Como o ciclo deve terminar: entradas, gastos e caixa"),
            ("extrato", "Gastos e entradas por categoria; /extrato 08/2026 para outro ciclo"),
            ("custodevida", "Quanto custa a vida básica; /custodevida 08/2026 para outro ciclo"),
            ("metas", "Progresso das metas"),
            ("orcamentos", "Orçamentos do ciclo"),
            ("exportar", "Planilha (CSV) do ciclo; /exportar 03/2026 para outro"),
        ],
    },
    CommandSection {
        title: "💳 Cartões e parcelas",
        commands: &[
            ("fatura", "Faturas dos cartões"),
            ("parcelas", "Compras parceladas e financiamentos: quanto falta"),
            ("cartoes", "Listar cartões"),
            ("recorrentes", "Listar e desativar recorrentes"),
        ],
    },
    CommandSection {
        title: "✏️ Corrigir",
        commands: &[
            ("desfazer", "Desfazer seu último lançamento"),
            ("ultimos", "Últimos lançamentos, com editar e apagar"),
            ("cancelar", "Cancelar o que está preenchendo"),
        ],
    },
    CommandSection {
        title: "⚙️ Cadastrar e configurar",
        commands: &[
            ("novaconta", "Cadastrar uma conta"),
            ("novocartao", "Cadastrar um cartão de crédito"),
            ("novameta", "Criar uma meta de economia"),
            ("novacategoria", "Criar uma categoria de gasto ou de entrada"),
            ("recorrente", "Criar um lançamento recorrente ou financiamento"),
            ("orcamento", "Definir ou remover o limite de uma categoria"),
            ("essenciais", "Marcar as categorias do custo de vida básico"),
            ("config", "Dia de início do ciclo e horários dos resumos"),
            ("contas", "Listar contas"),
            ("categorias", "Listar categorias"),
            ("ajuda", "Mostrar os comandos"),
        ],
    },
];

/// Every command, for `setMyCommands`; the menu follows the same order.
pub fn commands() -> Vec<(&'static str, &'static str)> {
    SECTIONS.iter().flat_map(|section| section.commands.iter().copied()).collect()
}

pub fn help_text() -> String {
    let blocks: Vec<String> = SECTIONS.iter().map(section_text).collect();
    format!("<b>Comandos</b>\n\n{}", blocks.join("\n\n"))
}

fn section_text(section: &CommandSection) -> String {
    let lines: Vec<String> = section
        .commands
        .iter()
        .map(|(command, description)| format!("/{command} — {description}"))
        .collect();
    format!("<b>{}</b>\n{}", section.title, lines.join("\n"))
}

pub fn welcome_text() -> String {
    format!("Olá! 👋 Vou registrar as finanças de vocês aqui.\n\n{}", help_text())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::flows::FormKind;

    #[test]
    fn help_lists_every_command_once() {
        let text = help_text();
        let all = commands();
        for (command, _) in &all {
            assert_eq!(text.matches(&format!("/{command} —")).count(), 1, "/{command} in {text}");
        }
        let unique: HashSet<&str> = all.iter().map(|(command, _)| *command).collect();
        assert_eq!(unique.len(), all.len(), "a command is listed twice");
        assert!(welcome_text().starts_with("Olá!"));
    }

    /// Every guided form must be reachable from the list.
    #[test]
    fn forms_are_in_the_list() {
        let listed: HashSet<&str> = commands().iter().map(|(command, _)| *command).collect();
        let typed = FormKind::ALL.into_iter().filter(|form| *form != FormKind::EditEntry);
        for form in typed {
            assert!(listed.contains(form.command()), "/{} is missing from /ajuda", form.command());
        }
    }

    /// Telegram refuses a command list with descriptions over 256 characters.
    #[test]
    fn descriptions_fit_the_menu() {
        for (command, description) in commands() {
            assert!((1..=256).contains(&description.len()), "/{command}: {description}");
            assert!(command.chars().all(|letter| letter.is_ascii_lowercase()), "/{command}");
        }
    }
}
