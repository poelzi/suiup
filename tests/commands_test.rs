mod common;

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use suiup::commands::{parse_component_with_version, SuiComponent};

    #[test]
    fn test_parse_component_with_version() -> Result<(), String> {
        let result = parse_component_with_version("sui")?;
        assert_eq!(result, (SuiComponent::Sui, None));

        let result = parse_component_with_version("sui testnet-v1.39.3")?;
        assert_eq!(
            result,
            (SuiComponent::Sui, Some("testnet-v1.39.3".to_string()))
        );

        let result = parse_component_with_version("sui-bridge devnet")?;
        assert_eq!(
            result,
            (SuiComponent::SuiBridge, Some("devnet".to_string()))
        );
        let result = parse_component_with_version("walrus")?;
        assert_eq!(result, (SuiComponent::Walrus, None));

        let result = parse_component_with_version("mvr")?;
        assert_eq!(result, (SuiComponent::Mvr, None));

        let result = parse_component_with_version("random");
        assert_eq!(result, Err("Invalid component name: random".to_string()));

        Ok(())
    }

    #[test]
    fn test_sui_component_display() {
        assert_eq!(SuiComponent::Sui.to_string(), "sui");
        assert_eq!(SuiComponent::SuiBridge.to_string(), "sui-bridge");
        assert_eq!(SuiComponent::SuiFaucet.to_string(), "sui-faucet");
        assert_eq!(SuiComponent::Mvr.to_string(), "mvr");
        assert_eq!(SuiComponent::Walrus.to_string(), "walrus");
    }
}
