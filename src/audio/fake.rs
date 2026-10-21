//! Root-error rollback test uses the extracted deterministic audio backend.
use crate::{AudioAccess, ControllerError};
pub(crate) use gr_root_audio::test_support::{Record, open};
#[test]
fn rollback_preserves_typed_creation_cause_and_audio_cleanup_failure() {
    for cleanup_failure in [false, true] {
        let (session, record) = open(1, AudioAccess::Samples);
        record.lock().unwrap().cleanup_failure = cleanup_failure;
        let mut audio = Some(session);
        let cause = ControllerError::MissingDeviceNode {
            target: crate::RealizationId::LINUX_UHID_USB,
            path: "synthetic-node".into(),
        };
        let error = crate::creation::finish::<()>(Err(cause.clone()), || {
            crate::creation::close_audio(&mut audio)
        })
        .unwrap_err();
        if cleanup_failure {
            match error {
                ControllerError::Cleanup {
                    cause: retained,
                    cleanup,
                } => {
                    assert_eq!(*retained, cause);
                    assert_eq!(cleanup.len(), 1);
                    assert!(cleanup[0].contains("audio cleanup failure"));
                }
                other => panic!("unexpected {other:?}"),
            }
        } else {
            assert_eq!(error, cause);
        }
        drop(audio);
        assert_eq!(record.lock().unwrap().closes, 1);
    }
}

#[test]
fn extracted_audio_error_mapping_preserves_typed_cause_and_cleanup_order() {
    let error = gr_root_audio::OpenError::Cleanup {
        cause: Box::new(gr_root_audio::OpenError::Unsupported {
            reason: "synthetic unsupported audio profile".into(),
        }),
        cleanup: vec![
            "synthetic worker cleanup".into(),
            "synthetic broker cleanup".into(),
        ],
    };
    assert_eq!(
        super::open_error(error),
        ControllerError::Cleanup {
            cause: Box::new(ControllerError::Unsupported {
                reason: "synthetic unsupported audio profile".into(),
            }),
            cleanup: vec![
                "synthetic worker cleanup".into(),
                "synthetic broker cleanup".into()
            ],
        }
    );
}
