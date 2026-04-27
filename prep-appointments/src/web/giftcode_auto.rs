//! Background task: automatically fetch and redeem gift codes for all accounts with recipients.

use std::time::Duration;

use crate::giftcode_api;
use crate::kingshot_api;

use super::giftcode_recipients;

/// Run one cycle: fetch codes, redeem for each account with recipients, skip already-redeemed codes.
pub async fn run_auto_redeem_cycle(data_dir: &str) {
    let account_servers = giftcode_recipients::list_accounts_with_recipients(data_dir);

    if account_servers.is_empty() {
        return;
    }

    let codes = match giftcode_api::fetch_giftcodes().await {
        Ok(c) => c,
        Err(_) => return,
    };

    if codes.is_empty() {
        return;
    }

    for (account, server) in account_servers {
        let player_ids = giftcode_recipients::load_recipients_internal(data_dir, &account, server);
        if player_ids.is_empty() {
            continue;
        }

        let redeemed = giftcode_recipients::load_redeemed_internal(data_dir, &account, server);
        let new_codes: Vec<String> = codes
            .iter()
            .map(|g| g.code.clone())
            .filter(|c| !redeemed.contains(&c.trim().to_uppercase()))
            .collect();

        for code in new_codes {
            for player_id in &player_ids {
                let result = kingshot_api::redeem_giftcode(player_id, &code).await;
                let _ = result;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            giftcode_recipients::add_redeemed_code_internal(data_dir, &account, server, &code).ok();
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
