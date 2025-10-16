#### Cambiador de Nombres

- **Nombres / Targets / Cooldown / Hora a cambiar:**  
  `src/features/server_specific/config/nickname_targets.rs` -> users targets IDs

- **Mensaje del bot al cambiar nombre y función que lo dispara:**  
  `src/features/server_specific/services/nickname_changer.rs`

#### Datos del server y canal

- **server id & channel id:**  
`src\features\server_specific\config\server_config.rs` -> cambiar server ID y channel ID
`src\infrastructure\discord_bot\bot.rs` -> cambiar my_server_id

