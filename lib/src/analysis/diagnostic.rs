use crate::analysis::analyzer::{Analyzer, Event, EventType};
use crate::analysis::imsi_exposure_classifier;
use crate::analysis::information_element::InformationElement;
use std::borrow::Cow;

pub struct DiagnosticAnalyzer;

impl Default for DiagnosticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticAnalyzer {
    pub fn new() -> Self {
        DiagnosticAnalyzer
    }
}

impl Analyzer for DiagnosticAnalyzer {
    fn get_name(&self) -> Cow<'_, str> {
        "Diagnostic detector for messages which might lead to IMSI exposure".into()
    }

    fn get_description(&self) -> Cow<'_, str> {
        "Catches any messages that may lead to IMSI Exposure. Can be quite noisy. \
        Useful as a diagnostic for finding out why an IMSI was sent or what \
        the reason for a reject message was. Not a useful indicator on its own \
        but a helpful diagnostic for understanding why another indicator was \
        triggered. Based on the list of IMSI exposing messages identified in \
        the Tucker et al. (NDSS 2025) paper."
            .into()
    }

    fn get_version(&self) -> u32 {
        2
    }

    fn analyze_information_element(
        &mut self,
        ie: &InformationElement,
        _packet_num: usize,
    ) -> Option<Event> {
        let classification = imsi_exposure_classifier::classify(ie)?;

        Some(Event {
            event_type: EventType::Informational,
            message: format!(
                "Diagnostic: {} ({:?}).",
                classification.description, classification.category
            ),
        })
    }
}
