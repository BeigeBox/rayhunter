//! Centralized classifier for messages that can cause IMSI exposure.
//!
//! Based on Tucker et al., "Detecting IMSI-Catchers by Characterizing Identity
//! Exposing Messages in Cellular Traffic" (NDSS 2025). This module provides a
//! shared classification function that tags parsed messages as IMSI-exposing,
//! for use by both the diagnostic analyzer and the ratio-based exposure analyzer.
//!
//! Currently covers LTE (4G) NAS and RRC messages. The UMTS/GSM/5G-NR variants
//! in InformationElement are stubs and will be classified once parsing support
//! is added.

use pycrate_rs::nas::emm::EMMMessage;
use pycrate_rs::nas::generated::emm::emm_attach_reject::EMMCauseEMMCause as AttachRejectEMMCause;
use pycrate_rs::nas::generated::emm::emm_detach_request_mt::EPSDetachTypeMTType;
use pycrate_rs::nas::generated::emm::emm_service_reject::EMMCauseEMMCause as ServiceRejectEMMCause;
use pycrate_rs::nas::generated::emm::emm_tracking_area_update_reject::EMMCauseEMMCause as TAURejectEMMCause;
use pycrate_rs::nas::NASMessage;
use telcom_parser::lte_rrc::{
    DL_DCCH_MessageType, DL_DCCH_MessageType_c1, PCCH_MessageType, PCCH_MessageType_c1,
    PagingUE_Identity, RRCConnectionReleaseCriticalExtensions,
    RRCConnectionReleaseCriticalExtensions_c1, RedirectedCarrierInfo,
};

use super::information_element::{InformationElement, LteInformationElement};

/// Categories of IMSI-exposing messages, following the taxonomy from Tucker et al.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImsiExposureCategory {
    /// NAS Identity Request (IMSI, IMEI, or IMEISV type)
    DirectIdentityRequest,
    /// Attach Reject with cause codes that force re-attach with IMSI
    AttachReject,
    /// Tracking Area Update Reject with cause codes that invalidate GUTI
    TauReject,
    /// Service Reject with causes that reset temporary identity
    ServiceReject,
    /// Authentication Reject forcing re-authentication with IMSI
    AuthenticationReject,
    /// Network-initiated detach with re-attach required
    DetachRequest,
    /// RRC Connection Release with redirect to 2G/3G cell
    ConnectionRedirect,
    /// Paging using IMSI instead of TMSI/GUTI
    PagingWithImsi,
}

/// Result of classifying a message for IMSI exposure potential.
#[derive(Debug, Clone)]
pub struct ImsiExposureClassification {
    pub category: ImsiExposureCategory,
    pub description: String,
}

/// Classify whether a parsed information element is an IMSI-exposing message.
///
/// Returns `Some(classification)` if the message is one of the known
/// IMSI-exposing types, or `None` if it is benign or not relevant.
pub fn classify(ie: &InformationElement) -> Option<ImsiExposureClassification> {
    match ie {
        InformationElement::LTE(lte_ie) => classify_lte(lte_ie),
        // UMTS, GSM, and 5G parsing are stubs; classify once implemented
        _ => None,
    }
}

/// Returns true if the message is relevant for total-connection counting
/// (i.e., it is a NAS or RRC message we should count in the denominator
/// of the exposure ratio). We count all successfully parsed LTE messages.
pub fn is_countable_message(ie: &InformationElement) -> bool {
    matches!(ie, InformationElement::LTE(_))
}

fn classify_lte(lte_ie: &LteInformationElement) -> Option<ImsiExposureClassification> {
    match lte_ie {
        LteInformationElement::NAS(nas_msg) => classify_nas(nas_msg),
        LteInformationElement::DlDcch(msg) => classify_dl_dcch(msg),
        LteInformationElement::PCCH(msg) => classify_pcch(msg),
        _ => None,
    }
}

fn classify_nas(nas_msg: &NASMessage) -> Option<ImsiExposureClassification> {
    match nas_msg {
        NASMessage::EMMMessage(emm_msg) => classify_emm(emm_msg),
        _ => None,
    }
}

fn classify_emm(emm_msg: &EMMMessage) -> Option<ImsiExposureClassification> {
    match emm_msg {
        // Direct Identity Request — the most obvious IMSI-exposing message
        EMMMessage::EMMIdentityRequest(request) => Some(ImsiExposureClassification {
            category: ImsiExposureCategory::DirectIdentityRequest,
            description: format!("EMM Identity Request ({:?})", request.id_type.inner),
        }),

        // Attach Reject with specific cause codes that force re-attach with IMSI
        EMMMessage::EMMAttachReject(reject) => {
            if matches!(
                reject.emm_cause.inner,
                AttachRejectEMMCause::IllegalUE
                    | AttachRejectEMMCause::IllegalME
                    | AttachRejectEMMCause::EPSServicesNotAllowed
                    | AttachRejectEMMCause::EPSServicesAndNonEPSServicesNotAllowed
                    | AttachRejectEMMCause::PLMNNotAllowed
                    | AttachRejectEMMCause::TrackingAreaNotAllowed
                    | AttachRejectEMMCause::RoamingNotAllowedInThisTrackingArea
                    | AttachRejectEMMCause::EPSServicesNotAllowedInThisPLMN
                    | AttachRejectEMMCause::NoSuitableCellsInTrackingArea
                    | AttachRejectEMMCause::RequestedServiceOptionNotAuthorizedInThisPLMN
            ) {
                Some(ImsiExposureClassification {
                    category: ImsiExposureCategory::AttachReject,
                    description: format!("EMM Attach Reject ({:?})", reject.emm_cause.inner),
                })
            } else {
                None
            }
        }

        // TAU Reject with cause codes that invalidate GUTI/TMSI
        EMMMessage::EMMTrackingAreaUpdateReject(reject) => {
            if matches!(
                reject.emm_cause.inner,
                TAURejectEMMCause::IllegalUE
                    | TAURejectEMMCause::IllegalME
                    | TAURejectEMMCause::EPSServicesNotAllowed
                    | TAURejectEMMCause::EPSServicesAndNonEPSServicesNotAllowed
                    | TAURejectEMMCause::TrackingAreaNotAllowed
                    | TAURejectEMMCause::EPSServicesNotAllowedInThisPLMN
                    | TAURejectEMMCause::RequestedServiceOptionNotAuthorizedInThisPLMN
            ) {
                Some(ImsiExposureClassification {
                    category: ImsiExposureCategory::TauReject,
                    description: format!(
                        "EMM TAU Reject ({:?})",
                        reject.emm_cause.inner
                    ),
                })
            } else {
                None
            }
        }

        // Service Reject with causes that reset temporary identity
        EMMMessage::EMMServiceReject(reject) => {
            if matches!(
                reject.emm_cause.inner,
                ServiceRejectEMMCause::IllegalUE
                    | ServiceRejectEMMCause::IllegalME
                    | ServiceRejectEMMCause::EPSServicesNotAllowed
                    | ServiceRejectEMMCause::UEIdentityCannotBeDerivedByTheNetwork
                    | ServiceRejectEMMCause::TrackingAreaNotAllowed
                    | ServiceRejectEMMCause::EPSServicesNotAllowedInThisPLMN
                    | ServiceRejectEMMCause::RequestedServiceOptionNotAuthorizedInThisPLMN
            ) {
                Some(ImsiExposureClassification {
                    category: ImsiExposureCategory::ServiceReject,
                    description: format!("EMM Service Reject ({:?})", reject.emm_cause.inner),
                })
            } else {
                None
            }
        }

        // Authentication Reject — forces re-authentication with IMSI
        EMMMessage::EMMAuthenticationReject(_) => Some(ImsiExposureClassification {
            category: ImsiExposureCategory::AuthenticationReject,
            description: "EMM Authentication Reject".to_string(),
        }),

        // Network-initiated detach (not IMSI detach type) — forces re-attach
        EMMMessage::EMMDetachRequestMT(req) => {
            if req.eps_detach_type.inner.typ != EPSDetachTypeMTType::IMSIDetach {
                Some(ImsiExposureClassification {
                    category: ImsiExposureCategory::DetachRequest,
                    description: format!(
                        "EMM Detach Request ({:?}:{:?})",
                        req.eps_detach_type.inner, req.emm_cause.inner
                    ),
                })
            } else {
                None
            }
        }

        _ => None,
    }
}

fn classify_dl_dcch(
    msg: &telcom_parser::lte_rrc::DL_DCCH_Message,
) -> Option<ImsiExposureClassification> {
    if let DL_DCCH_MessageType::C1(DL_DCCH_MessageType_c1::RrcConnectionRelease(release)) =
        &msg.message
    {
        if let RRCConnectionReleaseCriticalExtensions::C1(
            RRCConnectionReleaseCriticalExtensions_c1::RrcConnectionRelease_r8(r8_ies),
        ) = &release.critical_extensions
        {
            if let Some(carrier_info) = &r8_ies.redirected_carrier_info {
                if matches!(carrier_info, RedirectedCarrierInfo::Geran(_)) {
                    return Some(ImsiExposureClassification {
                        category: ImsiExposureCategory::ConnectionRedirect,
                        description: "RRC Connection Release with redirect to 2G (GERAN)"
                            .to_string(),
                    });
                }
            }
        }
    }
    None
}

fn classify_pcch(
    msg: &telcom_parser::lte_rrc::PCCH_Message,
) -> Option<ImsiExposureClassification> {
    if let PCCH_MessageType::C1(PCCH_MessageType_c1::Paging(paging)) = &msg.message {
        if let Some(ref records) = paging.paging_record_list {
            for record in &records.0 {
                if matches!(record.ue_identity, PagingUE_Identity::Imsi(_)) {
                    return Some(ImsiExposureClassification {
                        category: ImsiExposureCategory::PagingWithImsi,
                        description: "Paging with IMSI instead of S-TMSI".to_string(),
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_lte_messages_not_classified() {
        assert!(classify(&InformationElement::GSM).is_none());
        assert!(classify(&InformationElement::UMTS).is_none());
        assert!(classify(&InformationElement::FiveG).is_none());
    }

    #[test]
    fn test_non_lte_not_countable() {
        // Currently only LTE is countable since GSM/UMTS/5G are stubs
        assert!(!is_countable_message(&InformationElement::GSM));
        assert!(!is_countable_message(&InformationElement::UMTS));
        assert!(!is_countable_message(&InformationElement::FiveG));
    }
}
