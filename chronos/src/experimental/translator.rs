use crate::ChronosResult;
use std::sync::Arc;
use tardis_vortex::model::ModelHandle;
use tardis_vortex::{InferenceParams, Vortex};

/// The Universal Translator uses Vortex to translate text into different languages or personas.
#[derive(Debug)]
pub struct UniversalTranslator {
    vortex: Arc<Vortex>,
    model: ModelHandle,
}

impl UniversalTranslator {
    /// Create a new translator.
    #[must_use]
    pub fn new(vortex: Arc<Vortex>, model: ModelHandle) -> Self {
        Self { vortex, model }
    }

    /// Translate text to a target language or persona.
    ///
    /// The target can be a real language (e.g., "French") or a fictional style (e.g., "Yoda", "Shakespeare").
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails.
    pub async fn translate(&self, text: &str, target: &str) -> ChronosResult<String> {
        let prompt = format!(
            "Translate the following text to {target}. If the target is a style or persona, rewrite it in that style.\n\nText: \"{text}\"\n\nTranslation:"
        );

        let params = InferenceParams::creative()
            .with_max_tokens(256)
            .with_temperature(0.7);

        self.vortex
            .infer(self.model, &prompt, params)
            .await
            .map_err(crate::ChronosError::Vortex)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tardis_vortex::Vortex;

    #[tokio::test]
    async fn test_translator() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let handle = ModelHandle::new(999);

        // Set up mock inference
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            if prompt.contains("Translate the following text to Yoda") {
                Ok("Do or do not, there is no try.".to_string())
            } else {
                Ok("Unknown".to_string())
            }
        }));

        let translator = UniversalTranslator::new(vortex, handle);
        let result = translator.translate("Try to do it", "Yoda").await.unwrap();

        assert_eq!(result, "Do or do not, there is no try.");
    }
}
