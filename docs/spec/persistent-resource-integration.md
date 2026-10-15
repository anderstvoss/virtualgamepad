# Internal persistent-resource integration recipe

Step 9 provides an opt-in, typed terminal-snapshot wrapper in curated common
code. It adds no fields to ordinary ControllerSession and changes no gr-hid
runtime, provider or broker interface. The synthetic fifth family is the concrete
owner used to prove the seam; it is not a Wii package or device evidence.

A future Wii factory must construct/validate its own reference or imported typed
resource before calling common creation or opening any provider/worker. Exact
length, revision, protected bytes, calibration and model validation belong to that
factory. Its protocol owns the working value and deep-isolates it in Clone.
Unknown valid bytes survive. Host request decoding, access policy, calibrated
native interpretation and bounded replies remain controller-owned. No universal
address trait, mutable raw-memory field on every family, file policy or provider
protocol interpreter is introduced.

Families with resources implement the optional internal ResourceProtocol trait
(snapshot type, sticky resource_dirty and exact snapshot). ResourceSession wraps
only those handles. Caller order is: create, service including idle host requests,
close deliberately or observe terminal failure, inspect cleanup diagnostics,
resolve KeepSnapshot or Discard. Active sessions reject disposition. The first
explicit terminal choice wins; repeating or changing the request returns that
same outcome. Kept snapshots are immutable borrowed typed values that callers
may clone. Cleanup failure does not consume the protocol or snapshot. Drop,
consumer OPEN/CLOSE and host STOP/START never export or decide disposition.

Resource dirty means an accepted write changed working content at least once,
even if later writes restore boot bytes. Input dirty continues to mean pending
input work. A host write is accepted once before its ACK; definitely-unsent ACK
retries reuse the exact bytes. Uncertain delivery closes terminally with the last
accepted resource recoverable. Generation candidates and rejected typed edits
cannot mutate the working resource through shared mutable clone storage.

Step 8 owns topology/configuration edits in common/session.rs. Step 9 adds the
resource wrapper in its own module and only reads that session's existing
protocol/closed/diagnostics surfaces. A topology edit must retain resource bytes
and accepted command ownership; the joint queued-input/attachment/resource test
proves this boundary. A real Wii package supplies actual calibration and address
maps from research records, not the synthetic fixture's values or report IDs.

## Regression evidence

All names below are in registry::test_family::resource_tests unless stated.

| Minimum regression | Executable evidence |
| --- | --- |
| Default boot; bad length/version/calibration/model/protected bytes before open; unknown bytes | reference_and_import_validate_before_open_and_preserve_unknown_bytes |
| Per-instance isolation; reference immutability; deep clones | working_images_are_independent_and_reference_is_immutable |
| Legal/illegal ranges, overlap, boundaries and cross-region writes; read-after-write; volatile exclusion | synthetic_read_write_boundaries_classes_and_volatile_registers_are_exact (exhausts 256 x 256 selectors/write lengths) |
| Exact report-class success/errors | report_classes_get_exact_success_or_error_without_resource_mutation |
| Coherent calibration reads/native interpretation; restored identity, reset volatile state | calibration_and_identity_survive_restore_while_volatile_state_resets |
| Write accepted before ACK; exact retry without double write; sticky dirty independent of input dirty | host_write_is_accepted_once_before_identical_ack_retry_and_dirty_is_sticky |
| Rejected input/typed configuration candidates preserve accepted bytes | rejected_protocol_and_input_candidates_cannot_leak_or_undo_accepted_writes |
| Input queue saturation | full_input_queue_rejects_candidate_without_erasing_accepted_resource_write |
| Queued input, attachment and resource changes coexist without rewriting old command | queued_input_retry_keeps_bytes_across_resource_and_attachment_edits |
| Reply deadline retains accepted resource | reply_deadline_retains_prior_write_without_duplicate_mutation |
| Uncertain delivery, terminal snapshot and independent cleanup errors | uncertain_ack_and_failed_cleanup_keep_exact_terminal_snapshot_and_diagnostics |
| STOP/START/consumer lifecycle/Drop never export | stop_start_consumer_close_and_drop_never_export_or_resolve_disposition |
| First disposition wins, repeat close, failed caller export and retry | explicit_discard_wins_and_failed_caller_export_preserves_kept_snapshot; uncertain_ack_and_failed_cleanup_keep_exact_terminal_snapshot_and_diagnostics |
| Stateless no-overhead control | stateless_family_session_has_no_resource_slot_overhead (Xbox360 ordinary session versus zero-size resource slot) |

The seven original gr-hid persistent_resource regressions remain unchanged and
passing, including import_export_discard_and_ephemeral_recreation,
invalid_import_never_creates_transport,
read_write_and_every_report_class_receive_exact_success_or_error,
unsent_ack_retries_exactly_without_reapplying_write_or_rolling_back_memory,
uncertain_ack_and_failed_close_preserve_snapshot_and_export_retry,
explicit_close_failure_and_implicit_drop_do_not_call_exporter and
rejected_input_batch_does_not_mutate_dirty_protocol.

## Optional file helper

Skipped: no production caller currently needs a library file helper. This seam
returns typed data and never writes a file. The failed-export test injects a
caller failure; it is not an atomic-filesystem or fsync/crash-durability test.
When a caller needs persistence, implement and test same-filesystem temporary
replacement, permissions, rollback and explicitly scoped durability then. No Wii
address map or physical acceptance is claimed by these synthetic tests.
