# Test coverage — derived from architecture/spec

One row per test, bound to the **real** `// covers:`-tagged function in the source. This table is generated from those tags, so it is a 1:1 bijection: every tagged test is a row, and every row names a function that exists and carries that tag. Every test exercises real `flatroot` behavior — black-box runs of the built binary, live-distro-mirror integration, real export tools (`mkdwarfs`/`mksquashfs`), and an unprivileged user+mount-namespace sandbox.

## How these tests observe behavior

- **CLI surface & refusals** — the built binary via `assert_cmd`; an offline validation failure asserts the exact diagnostic, a non-zero exit, and that no network was touched (`fetching http` absent).
- **Resolution / index / parsing** — in-crate `#[cfg(test)]` plus `tests/` integration against catalogues populated through `Index::open_or_populate` and real package indices.
- **Install / export / sandbox / post-install** — full runs against live distro mirrors, real extraction, the user-namespace sandbox, and the real export tools; layer contents inspected with `dwarfsck`.
- **Output encodings** — the plain dotted `KEY=VALUE` form and its `--format json` bijection (empty container → zero bytes plain / `[]` json).

Run with `cargo test` / `cargo nextest run` (CI `test.yml`). Networked and privileged tests need live mirrors, FUSE, unprivileged user namespaces, Docker, and the export tools.

---

## CLI surface, parsing & dispatch — incl. `--match` flag + `owners_fold` (80)

| ID | Scenario | Covering test |
|---|---|---|
| `CLI-003` | --version prints crate version and exits zero | [cli_dispatch.rs](cli_dispatch.rs)::`version_flag_prints_crate_version_and_exits_zero` |
| `CLI-004` | Unknown subcommand is rejected | [cli_dispatch.rs](cli_dispatch.rs)::`unknown_subcommand_is_rejected` |
| `CLI-005` | Unknown global flag is rejected | [cli_dispatch.rs](cli_dispatch.rs)::`unknown_global_flag_is_rejected` |
| `CLI-007` | FLATROOT_ARG_FROM env fallback supplies --from | [cli_network.rs](cli_network.rs)::`env_from_drives_release_list` |
| `CLI-008` | --arch defaults to host architecture uname | [cli_network.rs](cli_network.rs)::`arch_default_is_host_uname` |
| `CLI-009` | --arch with each supported uname token parses | [cli_dispatch.rs](cli_dispatch.rs)::`arch_supported_uname_tokens_parse`, [cli_network.rs](cli_network.rs)::`arch_x86_alias_maps_to_i686_at_cli` |
| `CLI-010` | --arch with unsupported token is rejected | [cli_dispatch.rs](cli_dispatch.rs)::`arch_unsupported_token_is_rejected_before_network` |
| `CLI-012` | Analyze takes only the first --arch token from a comma list | [cli_network.rs](cli_network.rs)::`analyze_arch_comma_list_uses_first_token` |
| `CLI-013` | Search/Query/Release feed whole --arch string (no split) → fail on comma list | [cli_dispatch.rs](cli_dispatch.rs)::`search_arch_comma_list_bails_on_literal_string`, [cli_dispatch.rs](cli_dispatch.rs)::`query_arch_comma_list_bails_on_literal_string` |
| `CLI-014` | FLATROOT_ARG_ARCH env fallback | [cli_dispatch.rs](cli_dispatch.rs)::`arch_env_fallback_supplies_arch_and_loses_to_flag` |
| `CLI-015` | --http-retries default 3 + env FLATROOT_ARG_HTTP_RETRIES | [cli_dispatch.rs](cli_dispatch.rs)::`http_retries_default_is_three_in_help`, [cli_dispatch.rs](cli_dispatch.rs)::`http_retries_non_numeric_is_clap_error` |
| `CLI-016` | --verbose/-v controls stderr warn channel; default quiet | [cli_network.rs](cli_network.rs)::`verbose_flag_does_not_change_stdout` |
| `CLI-017` | FLATROOT_ARG_VERBOSE env fallback for verbose | [cli_network.rs](cli_network.rs)::`env_verbose_is_accepted_and_keeps_stdout_clean` |
| `CLI-019` | install requires --output, else dispatch-level bail | [cli_dispatch.rs](cli_dispatch.rs)::`install_missing_output_bails_at_dispatch` |
| `CLI-020` | install output via -o, --output, or FLATROOT_ARG_INSTALL_OUTPUT | [cli_network.rs](cli_network.rs)::`install_output_env_lands_rootfs_there`, [cli_network.rs](cli_network.rs)::`install_output_o_and_long_flag_agree` |
| `CLI-021` | install missing --from beats missing -o | [cli_dispatch.rs](cli_dispatch.rs)::`install_missing_from_fires_before_missing_output` |
| `CLI-022` | install --with recommends/suggests enum + env | [cli_network.rs](cli_network.rs)::`install_with_recommends_widens_closure` |
| `CLI-023` | install --postinstall default is all three phases | [cli_network.rs](cli_network.rs)::`install_postinstall_default_runs_all_three_phases` |
| `CLI-024` | install --postinstall subset selects fewer phases | [cli_network.rs](cli_network.rs)::`install_postinstall_subset_skips_hooks` |
| `CLI-029` | install --parallel/-p default 4 and env | [cli_network.rs](cli_network.rs)::`install_parallel_one_serializes_and_succeeds` |
| `CLI-030` | install --exclude comma list parsed, trimmed, empties filtered | [cli_network.rs](cli_network.rs)::`install_exclude_trims_and_drops_package` |
| `CLI-032` | search --type package\|library enum + env | [cli_network.rs](cli_network.rs)::`search_type_default_is_package` |
| `CLI-033` | search --format plain\|json, default plain, env | [cli_network.rs](cli_network.rs)::`search_format_default_is_plain`, [cli_network.rs](cli_network.rs)::`search_package_json_is_bijective_with_plain` |
| `CLI-034` | query reads SQL from positional FILE | [cli_network.rs](cli_network.rs)::`query_reads_sql_from_positional_file` |
| `CLI-037` | query --format plain\|json, default plain, env | [cli_network.rs](cli_network.rs)::`query_plain_drops_null_json_preserves_null`, [cli_network.rs](cli_network.rs)::`query_format_default_is_plain` |
| `CLI-040` | remote list --format plain\|json, default plain, env | [cli.rs](cli.rs)::`remote_list_default_format_is_plain`, [cli.rs](cli.rs)::`remote_list_json_format_is_bijective_with_plain` |
| `CLI-041` | remote with no action is rejected | [cli_dispatch.rs](cli_dispatch.rs)::`remote_without_action_is_rejected` |
| `CLI-044` | release list --format plain\|json, default plain, env | [cli_network.rs](cli_network.rs)::`release_list_json_is_bijective_and_carries_distro_fields` |
| `CLI-045` | release with no action is rejected | [cli_dispatch.rs](cli_dispatch.rs)::`release_without_action_is_rejected_before_from_check` |
| `CLI-046` | export requires two positionals (src_dir, output) | [cli_dispatch.rs](cli_dispatch.rs)::`export_requires_two_positionals` |
| `CLI-047` | export needs no --from and no --arch | [cli_network.rs](cli_network.rs)::`export_infers_arch_from_rootfs_without_from_or_arch` |
| `CLI-048` | export --format inference from output extension | [cli_network.rs](cli_network.rs)::`export_infers_format_from_extension` |
| `CLI-049` | export --format explicit values oci\|tar\|dwarfs\|sqfs + env | [cli_dispatch.rs](cli_dispatch.rs)::`export_unknown_format_is_rejected` |
| `CLI-050` | export --tag/-t required for oci, ignored otherwise, env | [cli_network.rs](cli_network.rs)::`export_oci_without_tag_fails`, [cli_network.rs](cli_network.rs)::`export_tar_ignores_tag` |
| `CLI-051` | analyze with no action is rejected | [cli_dispatch.rs](cli_dispatch.rs)::`analyze_without_action_is_rejected` |
| `CLI-053` | analyze trace --type package\|library + env | [cli_network.rs](cli_network.rs)::`analyze_trace_type_default_is_package` |
| `CLI-054` | analyze trace --format plain\|json\|tree, default plain, env | [cli_network.rs](cli_network.rs)::`analyze_trace_format_default_is_plain`, [cli_network.rs](cli_network.rs)::`analyze_trace_format_env_drives_json` |
| `CLI-055` | analyze trace --strategy declared\|linker, default both, env | [cli_network.rs](cli_network.rs)::`analyze_trace_strategy_declared_omits_linker_only_edges` |
| `CLI-056` | analyze trace --strategy= empty rejected at dispatch | [cli_dispatch.rs](cli_dispatch.rs)::`analyze_trace_empty_strategy_bails_before_from_required` |
| `CLI-058` | analyze trace --with widens declared closure, env | [cli_network.rs](cli_network.rs)::`analyze_trace_with_recommends_widens_declared_closure` |
| `CLI-061` | analyze trace framing on stderr for zero seeds | [cli_network.rs](cli_network.rs)::`analyze_trace_zero_seeds_framing_and_empty_stdout` |
| `CLI-062` | analyze trace framing single-seed shape | [cli_network.rs](cli_network.rs)::`analyze_trace_single_seed_framing_on_stderr_only` |
| `CLI-063` | analyze trace framing N-seed shape with >5 truncation | [cli_network.rs](cli_network.rs)::`analyze_trace_many_seeds_framing_truncates_above_five` |
| `CLI-064` | analyze trace trailing unresolved-soname warning on stderr | [cli_network.rs](cli_network.rs)::`analyze_trace_unresolved_warning_on_stderr_not_gated_by_verbose` |
| `CLI-065` | analyze trace happy path emits to stdout in chosen format | [cli_network.rs](cli_network.rs)::`analyze_trace_single_seed_framing_on_stderr_only` |
| `CLI-066` | analyze async; release/search/query in spawn_blocking | [cli_network.rs](cli_network.rs)::`spawn_blocking_task_error_propagates_nonzero` |
| `CLI-067` | Every FLATROOT_ARG_* env fallback wired and overridden by its flag | [cli_dispatch.rs](cli_dispatch.rs)::`arch_env_fallback_supplies_arch_and_loses_to_flag` |
| `CLI-069` | install happy path dispatches with fully-resolved InstallArgs | [cli_network.rs](cli_network.rs)::`install_full_flag_combination_succeeds` |
| `CLI-070` | --from format `<distro>:<release>[@<date>]` parsed downstream not at CLI | [cli_network.rs](cli_network.rs)::`from_malformed_date_surfaces_downstream` |
| `CLI-071` | Global flags accepted before or after the subcommand | [cli_dispatch.rs](cli_dispatch.rs)::`from_flag_is_top_level_accepted_before_subcommand_only` |
| `CLI-074` | http client build failure surfaces (cache dir unresolvable) | [cli_dispatch.rs](cli_dispatch.rs)::`cache_dir_unresolvable_propagates_nonzero` |
| `CLI-075` | owners_fold — `any` unions groups in first-seen order | [../src/parser.rs](../src/parser.rs)::`owners_fold_any_unions_preserving_first_seen_order` |
| `CLI-076` | owners_fold — `any` dedups a name across groups | [../src/parser.rs](../src/parser.rs)::`owners_fold_any_dedups_across_groups` |
| `CLI-077` | owners_fold — `all` keeps only the common owner | [../src/parser.rs](../src/parser.rs)::`owners_fold_all_keeps_only_the_common_owner` |
| `CLI-078` | owners_fold — `all` empty when no owner is common | [../src/parser.rs](../src/parser.rs)::`owners_fold_all_is_empty_when_no_owner_is_common` |
| `CLI-079` | owners_fold — `all` of a single group is identity | [../src/parser.rs](../src/parser.rs)::`owners_fold_all_single_group_is_identity` |
| `CLI-080` | owners_fold — `all` preserves first-group order | [../src/parser.rs](../src/parser.rs)::`owners_fold_all_preserves_first_group_order` |
| `CLI-081` | owners_fold — `all` drops a name in some but not all groups | [../src/parser.rs](../src/parser.rs)::`owners_fold_all_drops_partial_membership` |
| `CLI-082` | owners_fold — `all` with an empty group yields empty | [../src/parser.rs](../src/parser.rs)::`owners_fold_all_empty_group_yields_empty` |
| `CLI-083` | owners_fold — empty groups slice yields empty (totality) | [../src/parser.rs](../src/parser.rs)::`owners_fold_empty_groups_is_empty` |
| `CLI-084` | owners_fold — `any` dedups a repeat within a group | [../src/parser.rs](../src/parser.rs)::`owners_fold_any_tolerates_intragroup_dup` |
| `CLI-085` | `--match all --type package` refused (install), before network | [cli.rs](cli.rs)::`match_all_rejected_for_type_package` |
| `CLI-086` | `--match all --type package` refused (search) | [cli.rs](cli.rs)::`match_all_rejected_for_type_package` |
| `CLI-087` | `--match all --type package` refused (analyze trace) | [cli.rs](cli.rs)::`match_all_rejected_for_type_package` |
| `CLI-088` | `--type package --match any` passes the guard | [cli.rs](cli.rs)::`match_guard_is_narrow` |
| `CLI-089` | `--type library --match all` passes the guard | [cli.rs](cli.rs)::`match_guard_is_narrow` |
| `CLI-090` | `--type path --match all` passes the guard | [cli.rs](cli.rs)::`match_guard_is_narrow` |
| `CLI-091` | `--match` appears in --help (install/search/trace) | [cli.rs](cli.rs)::`match_flag_in_help_with_possible_values` |
| `CLI-092` | `--match` accepts `any` and `all` values | [cli.rs](cli.rs)::`match_flag_in_help_with_possible_values` |
| `CLI-093` | `--match` invalid value rejected | [cli.rs](cli.rs)::`match_invalid_value_rejected` |
| `CLI-094` | `FLATROOT_ARG_INSTALL_MATCH` env fallback honoured | [cli.rs](cli.rs)::`match_env_fallback_is_read` |
| `CLI-095` | `FLATROOT_ARG_SEARCH_MATCH` env fallback honoured | [cli.rs](cli.rs)::`match_env_fallback_is_read` |
| `CLI-096` | `FLATROOT_ARG_ANALYZE_TRACE_MATCH` env fallback honoured | [cli.rs](cli.rs)::`match_env_fallback_is_read` |
| `CLI-097` | explicit `--match` overrides the env var | [cli.rs](cli.rs)::`match_flag_overrides_env` |
| `CLI-098` | owners_fold — `all` dedups a repeat in the first group | [../src/parser.rs](../src/parser.rs)::`owners_fold_all_dedups_a_repeat_in_the_first_group` |
| `CLI-099` | Env-var vs flag precedence proven for every flag simultaneously | [crosscutting_cli.rs](crosscutting_cli.rs)::`http_retries_flag_overrides_env_and_env_reaches_parser`, [crosscutting_cli.rs](crosscutting_cli.rs)::`arch_flag_overrides_env_and_env_reaches_parser`, [crosscutting_cli.rs](crosscutting_cli.rs)::`from_flag_overrides_env_and_env_supplies_when_absent` |
| `CLI-100` | Search/Query/Release REJECT comma --arch; install splits; analyze first | [crosscutting_cli.rs](crosscutting_cli.rs)::`comma_arch_rejected_by_single_arch_commands`, [crosscutting_cli.rs](crosscutting_cli.rs)::`comma_arch_install_splits_and_analyze_takes_first_token` |
| `CLI-101` | One --http-retries value drives index fetch, archive download, AND release scrape | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`single_http_retries_value_flows_into_every_subsystem` |
| `CLI-102` | analyze async; release/search/query spawn_blocking; errors join-await correctly | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`spawn_blocking_and_async_failures_propagate_identically` |
| `CLI-105` | install required-flag check ordering: missing --from beats -o beats positionals | [cli_dispatch.rs](cli_dispatch.rs)::`install_required_flag_check_ordering` |

## Install pipeline — incl. `--match` seed resolution (67)

| ID | Scenario | Covering test |
|---|---|---|
| `INST-002` | Missing --from rejected before any work | [install_flags.rs](install_flags.rs)::`missing_from_rejected_before_any_work` |
| `INST-003` | Missing --output rejected with -o guidance | [install_flags.rs](install_flags.rs)::`missing_output_rejected_with_o_guidance` |
| `INST-005` | Env fallbacks drive a fully env-configured install | [workflow_idempotent.rs](workflow_idempotent.rs)::`env_fallbacks_drive_install_like_flags` |
| `INST-007` | --no-deps installs only named packages | [install_pipeline.rs](install_pipeline.rs)::`no_deps_prints_marker_and_skips_resolver` |
| `INST-008` | Full resolution seeds base+essential+user | [install_pipeline.rs](install_pipeline.rs)::`full_resolution_seeds_base_essential_user` |
| `INST-010` | --with suggests widens via Suggests/optdepends | [install_pipeline.rs](install_pipeline.rs)::`with_suggests_widens_set` |
| `INST-011` | --with recommends,suggests combines both | [install_pipeline.rs](install_pipeline.rs)::`with_recommends_suggests_combines_both` |
| `INST-012` | Default (no --with) omits recommends and suggests | [install_pipeline.rs](install_pipeline.rs)::`default_omits_recommends_and_suggests` |
| `INST-013` | --exclude drops a package and exclusively-transitive deps | [install_pipeline.rs](install_pipeline.rs)::`exclude_drops_named_and_private_deps` |
| `INST-014` | --exclude with multiple comma-separated names | [install_flags.rs](install_flags.rs)::`exclude_comma_list_with_spaces_parses`, [install_pipeline.rs](install_pipeline.rs)::`exclude_comma_list_parses_to_trimmed_set` |
| `INST-015` | Empty --exclude string yields no exclusions | [install_pipeline.rs](install_pipeline.rs)::`exclude_empty_string_is_no_exclusions` |
| `INST-016` | --postinstall=none skips all post-install phases | [install_pipeline.rs](install_pipeline.rs)::`postinstall_none_skips_phases_but_finishes_and_runs_postfix` |
| `INST-018` | --postinstall ldconfig runs only ldconfig phase | [install_pipeline.rs](install_pipeline.rs)::`postinstall_ldconfig_runs_only_ldconfig` |
| `INST-019` | --postinstall scripts runs stubs_install then scripts_run | [install_pipeline.rs](install_pipeline.rs)::`postinstall_scripts_runs_only_scripts` |
| `INST-020` | --postinstall hooks runs only cache hooks | [install_pipeline.rs](install_pipeline.rs)::`postinstall_hooks_runs_only_hooks` |
| `INST-021` | Default --postinstall runs all three in fixed order | [install_pipeline.rs](install_pipeline.rs)::`postinstall_default_runs_all_three_in_order` |
| `INST-022` | --postinstall token order does not change execution order | [install_pipeline.rs](install_pipeline.rs)::`postinstall_token_order_does_not_change_execution_order` |
| `INST-023` | Post-install fails clearly when userns unavailable | [install_flags.rs](install_flags.rs)::`postinstall_userns_unavailable_bails_with_remedy_manifest_already_written` |
| `INST-024` | ldconfig phase auto-skips on musl (Alpine) | [install_pipeline.rs](install_pipeline.rs)::`ldconfig_auto_skips_on_alpine_musl` |
| `INST-025` | Multiarch install runs pipeline once per arch into one root | [install_pipeline.rs](install_pipeline.rs)::`multiarch_runs_pipeline_per_arch_one_postinstall` |
| `INST-026` | Single-arch install omits the '=== arch ===' header | [install_pipeline.rs](install_pipeline.rs)::`single_arch_omits_header` |
| `INST-027` | Unsupported --arch token rejected | [install_flags.rs](install_flags.rs)::`unsupported_arch_token_rejected` |
| `INST-028` | --arch accepts both 'i686' and 'x86' for 32-bit Intel | [install_pipeline.rs](install_pipeline.rs)::`arch_x86_alias_proceeds_as_i686` |
| `INST-033` | Up-to-date requires matching version AND checksum | [workflow_idempotent.rs](workflow_idempotent.rs)::`checksum_mismatch_forces_reextract_with_replacing_diagnostic` |
| `INST-034` | Reused up-to-date record carries prior source/url/files | [workflow_idempotent.rs](workflow_idempotent.rs)::`reused_record_keeps_prior_url` |
| `INST-036` | Manifest written only after all arches extract successfully | [install_pipeline.rs](install_pipeline.rs)::`manifest_written_only_after_all_arches_succeed` |
| `INST-037` | Post-install + Postfix gated on total_extracted across arches | [workflow_idempotent.rs](workflow_idempotent.rs)::`multiarch_reinstall_all_current_nothing_to_do` |
| `INST-038` | Postfix permission fixup makes mode-0 files 0644 | [install_flags.rs](install_flags.rs)::`postfix_fixes_mode_zero_files_only` |
| `INST-039` | Postfix fails the run if a correction cannot complete | [install_flags.rs](install_flags.rs)::`postfix_fails_when_correction_cannot_complete` |
| `INST-040` | Postfix runs even when --postinstall=none (extraction occurred) | [install_pipeline.rs](install_pipeline.rs)::`postinstall_none_skips_phases_but_finishes_and_runs_postfix` |
| `INST-041` | Postfix skipped when nothing was extracted | [workflow_idempotent.rs](workflow_idempotent.rs)::`postfix_skipped_when_nothing_extracted_modes_unchanged` |
| `INST-042` | Output directory created when absent | [install_pipeline.rs](install_pipeline.rs)::`output_directory_created_when_absent` |
| `INST-043` | Existing non-flatroot contents left in place (overwrite file-by-file) | [install_pipeline.rs](install_pipeline.rs)::`existing_non_flatroot_contents_left_in_place` |
| `INST-044` | Resolved package missing from DB after resolution is a hard error | [../src/commands/install.rs](../src/commands/install.rs)::`pkg_row_required_bails_when_resolved_name_is_absent_from_db` |
| `INST-045` | --parallel controls download concurrency; 1 forces sequential | [install_pipeline.rs](install_pipeline.rs)::`parallel_one_forces_sequential_and_succeeds` |
| `INST-046` | --http-retries threads through to index fetch and downloads | [install_pipeline.rs](install_pipeline.rs)::`http_retries_threads_through_to_index_and_downloads` |
| `INST-047` | Index fetch failure (unreachable mirror) surfaces as error | [install_pipeline.rs](install_pipeline.rs)::`index_fetch_failure_is_nonzero_no_manifest` |
| `INST-048` | Cached index reused without refetch when current | [workflow_idempotent.rs](workflow_idempotent.rs)::`index_db_cache_reused_without_refetch` |
| `INST-049` | ArchContext prints index diagnostics and creates download cache dir | [install_pipeline.rs](install_pipeline.rs)::`archcontext_prints_index_diagnostics_and_creates_cache_dir` |
| `INST-050` | Snapshot-dated source (@date) treated as distinct, pinned source | [install_pipeline.rs](install_pipeline.rs)::`snapshot_dated_source_recorded_verbatim` |
| `INST-051` | Empty resolution writes unchanged manifest, nothing extracted | [workflow_idempotent.rs](workflow_idempotent.rs)::`zero_extraction_reinstall_prints_all_current_line` |
| `INST-052` | merge supersedes same (name,arch) records and recomputes sources/archs | [workflow_idempotent.rs](workflow_idempotent.rs)::`merge_supersedes_record_and_shows_newer_version` |
| `INST-053` | Multiarch with one arch unsupported fails that arch's context open | [install_pipeline.rs](install_pipeline.rs)::`multiarch_later_arch_failure_is_nonzero_prior_manifest_intact` |
| `INST-054` | primary_arch fallback for post-install uses first requested arch | [install_pipeline.rs](install_pipeline.rs)::`primary_arch_fallback_uses_first_requested_arch` |
| `INST-055` | 'Done.' summary reports total = full manifest package count | [install_pipeline.rs](install_pipeline.rs)::`done_summary_counts_full_merged_manifest` |
| `INST-056` | Mutating install silent on stdout (output contract) | [install_pipeline.rs](install_pipeline.rs)::`install_is_silent_on_stdout` |
| `INST-057` | Extraction order follows resolver dependency-first order | [install_pipeline.rs](install_pipeline.rs)::`extraction_order_preserves_merged_usr_symlink` |
| `INST-059` | Download skips archives already in checksum-verified cache | [workflow_idempotent.rs](workflow_idempotent.rs)::`corrupt_cached_archive_is_refetched` |
| `INST-060` | Downloaded archive missing from the map silently skipped during extraction | [../src/commands/install.rs](../src/commands/install.rs)::`downloaded_archive_is_none_for_an_unmapped_package` |
| `INST-061` | install `--type path --match all` picks the common owner | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_path_intersect_picks_the_common_owner` |
| `INST-062` | install `--match all` disjoint owners errors naming patterns | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_path_intersect_disjoint_owners_errors` |
| `INST-063` | install `--type path --match any` unions owners (explicit) | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_path_match_any_unions_owners` |
| `INST-064` | install `--type library --match all` picks the common owner | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_library_intersect_picks_common_owner` |
| `INST-065` | install `--type library --match all` disjoint errors | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_library_intersect_disjoint_errors` |
| `INST-066` | install `--match all` single pattern is identity | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_intersect_single_pattern_identity` |
| `INST-067` | install `--match all` empty pattern errors per-pattern (not intersection) | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_intersect_empty_pattern_errors_per_pattern` |
| `INST-068` | install `--match all` intersects glob-expanded owners | [../src/commands/install.rs](../src/commands/install.rs)::`requested_resolve_intersect_with_glob_expansion` |
| `INST-070` | install `--type path --match all` installs resolved package + closure (live) | [cli_network.rs](cli_network.rs)::`install_path_match_all_installs_resolved_package` |
| `INST-071` | install `--type path --match all --no-deps` installs only the resolved package (live) | [cli_network.rs](cli_network.rs)::`install_path_match_all_no_deps_installs_only_resolved` |
| `INST-072` | install `--type library --match all` multiarch resolves per-arch (live) | [cli_network.rs](cli_network.rs)::`install_library_match_all_multiarch_per_arch` |
| `INST-073` | Downloader skips a needs-extract archive missing from the downloaded map | [install_pipeline.rs](install_pipeline.rs)::`downloader_hardfails_before_install_defensive_continue` |
| `INST-074` | Multiarch install: manifest written ONCE after every arch succeeds | [install_pipeline.rs](install_pipeline.rs)::`multiarch_manifest_written_once_spanning_all_arches` |
| `INST-075` | Post-install + Postfix gated on total_extracted summed across arches | [install_pipeline.rs](install_pipeline.rs)::`postinstall_gated_on_summed_extraction_runs_once` |
| `INST-078` | --no-deps validates each name exists, bails on first unknown, all 10 distros | [install_pipeline.rs](install_pipeline.rs)::`no_deps_bails_on_unknown_name_across_formats` |
| `INST-079` | Resolved-but-missing-from-DB is a hard error in the Downloader | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`downloader_fetch_bails_on_name_absent_from_db` |
| `INST-080` | install `--type path --match all` on apk refused (apk precedes combine) (live) | [cli_network.rs](cli_network.rs)::`install_path_match_all_on_apk_refused` |
| `INST-081` | install `--match all`: `Resolved` echoes on stderr, not stdout (live) | [cli_network.rs](cli_network.rs)::`install_path_match_all_installs_resolved_package` |

## Dependency resolver (57)

| ID | Scenario | Covering test |
|---|---|---|
| `RES-001` | Simple linear dependency chain expands fully, dep-first order | [resolver_features.rs](resolver_features.rs)::`linear_chain_expands_fully_in_dep_first_bfs_order` |
| `RES-002` | analyze trace renders closure as tree with edges/kinds/picked-alt/constraint | [resolver_features.rs](resolver_features.rs)::`trace_renders_chain_with_edge_kinds_and_leaf_has_zero_edges` |
| `RES-004` | Circular dependency does not loop, both nodes once | [resolver_features.rs](resolver_features.rs)::`circular_dependency_terminates_with_both_nodes_once` |
| `RES-006` | Provider selection is stable via providers().first ordering | [resolver_features.rs](resolver_features.rs)::`provider_selection_is_deterministic_across_runs` |
| `RES-007` | Alternatives group: first-fit left-to-right by author order | [resolver_features.rs](resolver_features.rs)::`alternatives_first_fit_left_to_right_records_picked` |
| `RES-008` | Alternatives: first fails version, falls through to next | [resolver_features.rs](resolver_features.rs)::`alternatives_fall_through_when_first_fails_version` |
| `RES-010` | Unsatisfiable hard constraint, no alt: dep still named (visible missing) | [resolver_features.rs](resolver_features.rs)::`unsatisfiable_hard_constraint_no_alt_still_names_dep` |
| `RES-011` | Original version constraint preserved verbatim on trace edge | [resolver_features.rs](resolver_features.rs)::`single_alt_slot_preserves_constraint_verbatim_and_picked_none` |
| `RES-012` | provides-with-version judged against advertised provides version | [resolver_features.rs](resolver_features.rs)::`provides_with_version_judged_against_advertised_version` |
| `RES-013` | File-path dependency (RPM /path) resolved via path index | [resolver_features.rs](resolver_features.rs)::`file_path_dependency_resolved_via_path_index` |
| `RES-014` | File-path dep with no path index yields nothing (Alpine) | [resolver_features.rs](resolver_features.rs)::`file_path_dep_with_no_path_index_hard_bails` |
| `RES-015` | File-path dep where PathIndex::open returns None yields no candidate | [resolver_features.rs](resolver_features.rs)::`file_path_dep_with_empty_path_index_yields_no_candidate` |
| `RES-020` | --with recommends,suggests includes both tiers | [resolver_features.rs](resolver_features.rs)::`with_recommends_and_suggests_includes_both_tiers` |
| `RES-021` | Advisory dep that cannot be satisfied is silently dropped | [resolver_features.rs](resolver_features.rs)::`advisory_dep_that_cannot_be_satisfied_is_silently_dropped` |
| `RES-022` | --exclude prunes a package and exclusively-transitive deps | [resolver_features.rs](resolver_features.rs)::`exclude_prunes_package_and_exclusively_transitive_deps` |
| `RES-023` | --exclude of a package reachable via another path keeps it | [resolver_features.rs](resolver_features.rs)::`exclude_keeps_package_reachable_via_another_path` |
| `RES-024` | --exclude applied after virtual resolution (excluded real_name skipped) | [resolver_features.rs](resolver_features.rs)::`exclude_applied_after_virtual_resolution` |
| `RES-025` | Conflicts declaration warns but includes both | [resolver_features.rs](resolver_features.rs)::`conflicts_declaration_includes_both_packages` |
| `RES-026` | Breaks with applicable version constraint warns but includes both | [resolver_features.rs](resolver_features.rs)::`breaks_with_applicable_version_constraint_includes_both` |
| `RES-027` | Breaks with non-applicable constraint registers no clash | [resolver_features.rs](resolver_features.rs)::`breaks_with_non_applicable_version_constraint_includes_both` |
| `RES-028` | RPM rich-dep If: payload installed when condition present | [resolver_features.rs](resolver_features.rs)::`rich_dep_if_payload_installed_and_edge_is_richif` |
| `RES-029` | RPM rich-dep If: payload NOT installed when condition absent | [resolver_features.rs](resolver_features.rs)::`rich_dep_if_payload_absent_when_condition_absent` |
| `RES-030` | RPM rich-dep Unless: fallback when guard absent | [resolver_features.rs](resolver_features.rs)::`rich_dep_unless_fallback_installed_and_edge_is_richunless` |
| `RES-031` | RPM rich-dep Unless: fallback NOT when guard present | [resolver_features.rs](resolver_features.rs)::`rich_dep_unless_fallback_absent_when_guard_present` |
| `RES-032` | Rich-dep fixpoint cascades across passes | [resolver_features.rs](resolver_features.rs)::`rich_dep_fixpoint_cascades_across_passes` |
| `RES-033` | Rich-dep payload brought in with full transitive hard-dep closure | [resolver_features.rs](resolver_features.rs)::`rich_dep_payload_brings_full_transitive_hard_closure` |
| `RES-034` | Rich-dep payload with unsatisfiable hard dep aborts (strict) | [resolver_features.rs](resolver_features.rs)::`rich_dep_payload_with_unsatisfiable_hard_dep_aborts_strictly` |
| `RES-035` | Rich-dep Or prefers already-satisfied branch | [resolver_features.rs](resolver_features.rs)::`rich_dep_or_prefers_already_satisfied_branch` |
| `RES-036` | Rich-dep And unions both sides' payloads | [resolver_features.rs](resolver_features.rs)::`rich_dep_and_unions_both_payload_sides` |
| `RES-037` | Rich-dep With/Without payload uses left side only | [resolver_features.rs](resolver_features.rs)::`rich_dep_with_without_payload_uses_left_only` |
| `RES-038` | Rich-dep condition with version judged against present package | [resolver_features.rs](resolver_features.rs)::`rich_dep_condition_with_version_judged_against_present_package` |
| `RES-039` | Rich-dep condition satisfied by a virtual provider | [resolver_features.rs](resolver_features.rs)::`rich_dep_condition_satisfied_by_virtual_provider` |
| `RES-040` | Nested rich-dep condition with unmet precondition vacuously true | [resolver_features.rs](resolver_features.rs)::`rich_dep_nested_condition_unmet_precondition_is_vacuously_true` |
| `RES-041` | Rich-dep fixpoint only evaluates rich deps of installed packages | [resolver_features.rs](resolver_features.rs)::`rich_dep_fixpoint_only_evaluates_rich_deps_of_installed_packages` |
| `RES-042` | Rich-dep payload resolving to absent name skipped, not fatal | [resolver_features.rs](resolver_features.rs)::`rich_dep_payload_resolving_to_absent_name_is_skipped_not_fatal` |
| `RES-043` | Alpine install-if: auto-installed only when ALL triggers present | [resolver_features.rs](resolver_features.rs)::`install_if_triggers_only_when_all_conditions_present` |
| `RES-044` | Alpine install-if: NOT triggered when partially met | [resolver_features.rs](resolver_features.rs)::`install_if_not_triggered_when_partially_met` |
| `RES-045` | Alpine install-if fixpoint cascade | [resolver_features.rs](resolver_features.rs)::`install_if_fixpoint_cascades_across_passes` |
| `RES-046` | Alpine install-if trigger with unsatisfiable hard dep dropped w/ warning | [resolver_features.rs](resolver_features.rs)::`install_if_trigger_with_unsatisfiable_hard_dep_dropped_and_build_succeeds` |
| `RES-047` | Alpine install-if: one trigger's skip doesn't block others | [resolver_features.rs](resolver_features.rs)::`install_if_one_trigger_skip_does_not_block_others` |
| `RES-048` | Alpine install-if trigger satisfied by a virtual provider | [resolver_features.rs](resolver_features.rs)::`install_if_trigger_satisfied_by_virtual_provider` |
| `RES-049` | Alpine install-if trigger with version pin (~) matched on bare name | [resolver_features.rs](resolver_features.rs)::`install_if_trigger_with_version_pin_matches_on_bare_name` |
| `RES-050` | Alpine install-if trigger closure unsatisfiable rolls back atomically | [resolver_features.rs](resolver_features.rs)::`install_if_trigger_with_unsatisfiable_hard_dep_dropped_and_build_succeeds` |
| `RES-051` | Transitive resolve of already-visited package is a no-op (true) | [resolver_features.rs](resolver_features.rs)::`transitive_resolve_of_already_visited_is_noop` |
| `RES-052` | Transitive resolve of unknown name in conditional context skipped silently | [resolver_features.rs](resolver_features.rs)::`transitive_resolve_of_unknown_name_in_conditional_context_is_silent` |
| `RES-053` | Three-phase ordering: main BFS before rich-dep then install-if | [resolver_features.rs](resolver_features.rs)::`three_phase_ordering_walk_then_rich_dep_then_install_if` |
| `RES-054` | analyze trace rejects empty seed slice | [resolver_features.rs](resolver_features.rs)::`deptree_walk_rejects_empty_seed_slice` |
| `RES-056` | DepTree roots reflect resolved (post-virtual) names in input order | [resolver_features.rs](resolver_features.rs)::`deptree_roots_reflect_resolved_post_virtual_names_in_input_order` |
| `RES-057` | DepTree node carries index name/version; leaves have empty edges | [resolver_features.rs](resolver_features.rs)::`trace_renders_chain_with_edge_kinds_and_leaf_has_zero_edges` |
| `RES-058` | DepTree::walk bails when resolved name missing from index at node-build | [resolver_features.rs](resolver_features.rs)::`deptree_walk_bails_when_resolved_name_missing_from_index_at_node_build` |
| `RES-059` | analyze trace declared vs linker changes traced edges | [resolver_features.rs](resolver_features.rs)::`analyze_strategy_declared_vs_linker_changes_traced_edges` |
| `RES-061` | install resolver order honors filesystem/usrmerge-first BFS layering | [resolver_features.rs](resolver_features.rs)::`resolver_order_honors_filesystem_first_bfs_layering` |
| `RES-062` | Empty dependency line / leaf package resolves to just itself | [resolver_features.rs](resolver_features.rs)::`leaf_package_resolves_to_just_itself_with_no_edges` |
| `RES-063` | FLATROOT_ARG_* env fallback drives resolver flags identically | [resolver_features.rs](resolver_features.rs)::`flatroot_arg_env_fallback_drives_resolver_identically_to_flags` |
| `RES-064` | --exclude of a top-level requested package drops it before resolution | [resolver_features.rs](resolver_features.rs)::`exclude_of_top_level_requested_package_drops_it_at_queue_head` |
| `RES-065` | Conflicts/Breaks version constraint evaluated at registration moment | [resolver_features.rs](resolver_features.rs)::`breaks_with_applicable_version_constraint_includes_both`, [resolver_features.rs](resolver_features.rs)::`breaks_with_non_applicable_version_constraint_includes_both` |
| `RES-066` | Per-distro version comparator drives every constraint check | [resolver_features.rs](resolver_features.rs)::`per_distro_comparator_drives_constraint_checks` |

## Index: search, library & path matching — incl. `--match` filter (69)

| ID | Scenario | Covering test |
|---|---|---|
| `IDX-001` | search package single literal pattern returns matching package | [search.rs](search.rs)::`search_package_single_literal_plain_shape` |
| `IDX-003` | search package glob is case-insensitive | [search.rs](search.rs)::`search_package_glob_case_insensitive` |
| `IDX-006` | search package empty match emits '[]' in json | [search.rs](search.rs)::`search_package_empty_match_json_is_empty_array`, [search.rs](search.rs)::`search_package_empty_match_plain_is_zero_bytes` |
| `IDX-007` | search package plain↔json bijection for non-empty result | [search.rs](search.rs)::`search_package_plain_json_bijection` |
| `IDX-008` | search package --format flag value validation | [search.rs](search.rs)::`search_invalid_format_rejected` |
| `IDX-009` | search package format env fallback FLATROOT_ARG_SEARCH_FORMAT | [search.rs](search.rs)::`search_format_env_fallback_and_flag_override` |
| `IDX-010` | search --type env fallback FLATROOT_ARG_SEARCH_TYPE selects library | [search.rs](search.rs)::`search_type_env_fallback_routes_to_library` |
| `IDX-013` | search library: Debian-style match from path-index leg only | [search.rs](search.rs)::`search_library_debian_and_ubuntu_path_index` |
| `IDX-015` | search library: RPM mangled provides matched only with trailing-* pattern | [search.rs](search.rs)::`search_library_rpm_family_mangled_provides` |
| `IDX-016` | search library: Arch =-decorated provides version stripped | [search.rs](search.rs)::`search_library_pacman_family_eq_decorated_provides` |
| `IDX-017` | search library filters out non-library-shaped provides | [search.rs](search.rs)::`search_library_filters_out_non_library_provides` |
| `IDX-018` | search library dedups same (package, library) across both legs | [search.rs](search.rs)::`search_library_dedups_pair_across_legs` |
| `IDX-020` | search library multi-pattern union, sort by (library,name), dedup | [search.rs](search.rs)::`search_library_multi_pattern_ordered_and_deduped` |
| `IDX-021` | search library path-index leg is case-SENSITIVE | [search.rs](search.rs)::`search_library_path_index_is_case_insensitive` |
| `IDX-022` | search library: missing path index contributes nothing (no error) | [search.rs](search.rs)::`search_library_alpine_missing_path_index_provides_only` |
| `IDX-023` | search library empty match: zero bytes plain / [] json | [search.rs](search.rs)::`search_library_empty_match_formats` |
| `IDX-024` | search library plain↔json bijection includes the library field | [search.rs](search.rs)::`search_library_plain_json_bijection_with_library_field` |
| `IDX-025` | search library reports inconsistency when path-index pkg missing from table | [library_match.rs](library_match.rs)::`library_glob_path_index_pkg_missing_from_table_is_inconsistency` |
| `IDX-026` | query from stdin (no FILE) returns rows as plain KEY=VALUE | [query.rs](query.rs)::`query_stdin_multi_column_plain_rows` |
| `IDX-027` | query from stdin via explicit '-' sentinel | [query.rs](query.rs)::`query_dash_sentinel_reads_stdin` |
| `IDX-028` | query from a SQL file path | [query.rs](query.rs)::`query_reads_from_file_path` |
| `IDX-030` | query with invalid SQL fails at prepare | [query.rs](query.rs)::`query_invalid_sql_fails_at_prepare` |
| `IDX-031` | query NULL columns dropped in plain, preserved as null in json | [query.rs](query.rs)::`query_null_dropped_in_plain_preserved_in_json` |
| `IDX-032` | query maps SQLite value types: Integer, Real, Text, Blob, Null | [query.rs](query.rs)::`query_maps_all_sqlite_value_types` |
| `IDX-033` | query empty result set: zero bytes plain / [] json | [query.rs](query.rs)::`query_empty_result_formats` |
| `IDX-034` | query plain↔json bijection for multi-column rows | [query.rs](query.rs)::`query_multi_column_plain_json_bijection` |
| `IDX-035` | query format env fallback FLATROOT_ARG_QUERY_FORMAT | [query.rs](query.rs)::`query_format_env_fallback_json` |
| `IDX-036` | query column with no name falls back to '?' | [query.rs](query.rs)::`query_unaliased_expression_column_keys_under_its_expression_text` |
| `IDX-037` | query a non-SELECT statement (DML) behaves as zero-row read | [query.rs](query.rs)::`query_dml_statement_is_zero_row_read` |
| `IDX-038` | query exercises the registered glob_bash scalar function | [library_match.rs](library_match.rs)::`read_connection_has_glob_bash_scalar_function`, [query.rs](query.rs)::`query_glob_bash_function_is_registered_on_read_connection` |
| `IDX-039` | query exercises the version_compare collation on the read connection | [library_match.rs](library_match.rs)::`read_connection_has_version_compare_collation_rpm`, [query.rs](query.rs)::`query_version_compare_collation_is_registered_on_read_connection` |
| `IDX-040` | index cache fast-path reuse within TTL ('cached: <file>' on stderr) | [library_match.rs](library_match.rs)::`cache_fresh_fast_path_reuses_without_repopulating` |
| `IDX-041` | index cache rebuild after TTL expiry | [library_match.rs](library_match.rs)::`cache_rebuilds_after_ttl_expiry` |
| `IDX-042` | concurrent index population serialized by per-cache-key flock | [library_match.rs](library_match.rs)::`cache_concurrent_populate_serialized_by_flock` |
| `IDX-043` | index populate failure leaves no usable cache (tmp+rename atomicity) | [library_match.rs](library_match.rs)::`cache_populate_failure_leaves_no_usable_cache` |
| `IDX-044` | cache_key with slash sanitized into db filename | [library_match.rs](library_match.rs)::`cache_key_slash_sanitized_and_db_lands_under_index_dir` |
| `IDX-045` | FLATROOT_CACHE_HOME redirects the index store location | [library_match.rs](library_match.rs)::`cache_key_slash_sanitized_and_db_lands_under_index_dir`, [library_match.rs](library_match.rs)::`open_reads_back_db_written_under_redirected_cache_home` |
| `IDX-046` | path index round-trip: query a file to its owning package | [path_index_query.rs](path_index_query.rs)::`query_resolves_known_path_and_unknown_returns_none` |
| `IDX-047` | path index query for a path with no '/' returns None | [path_index_query.rs](path_index_query.rs)::`query_path_without_slash_returns_none` |
| `IDX-048` | path index query_glob basename matching, union + sort + dedup | [path_index_query.rs](path_index_query.rs)::`query_glob_basename_union_sort_and_empty` |
| `IDX-049` | path index query_glob '?' matches exactly one char | [path_index_query.rs](path_index_query.rs)::`query_glob_question_mark_matches_exactly_one_char` |
| `IDX-050` | path index query_glob dedups same (filename,package) across dirs | [path_index_query.rs](path_index_query.rs)::`query_glob_dedups_same_filename_pkg_across_dirs` |
| `IDX-051` | path index query_glob with invalid glob pattern errors | [path_index_query.rs](path_index_query.rs)::`query_glob_invalid_pattern_errors` |
| `IDX-052` | path index open is process-cached (parse once per file) | [path_index_query.rs](path_index_query.rs)::`open_is_process_cached_returns_same_arc`, [path_index_query.rs](path_index_query.rs)::`open_missing_index_returns_none` |
| `IDX-053` | path index rejects a non-record file by magic bytes | [path_index_query.rs](path_index_query.rs)::`parse_rejects_bad_magic` |
| `IDX-054` | path index rejects an unsupported layout version | [path_index_query.rs](path_index_query.rs)::`parse_rejects_unsupported_version` |
| `IDX-055` | path index rejects a file smaller than the header | [path_index_query.rs](path_index_query.rs)::`parse_rejects_file_smaller_than_header` |
| `IDX-056` | path index rejects truncated string table / entries | [path_index_query.rs](path_index_query.rs)::`parse_rejects_truncated_string_table`, [path_index_query.rs](path_index_query.rs)::`parse_rejects_truncated_entries` |
| `IDX-057` | path index rejects an out-of-range package id during lookup | [path_index_query.rs](path_index_query.rs)::`lookup_rejects_out_of_range_package_id` |
| `IDX-058` | path index of_db derives sibling .pathidx from the .db path | [path_index_query.rs](path_index_query.rs)::`of_db_derives_sibling_pathidx_extension`, [path_index_query.rs](path_index_query.rs)::`of_db_round_trips_through_a_real_index_file` |
| `IDX-059` | path index builder pools strings and dedups entries at finalize | [path_index_query.rs](path_index_query.rs)::`builder_pools_strings_remaps_ids_and_dedups_entries` |
| `IDX-060` | path index ingest of XML filelists folds packages and files | [path_index_query.rs](path_index_query.rs)::`ingest_xml_filelists_folds_packages_and_files`, [path_index_query.rs](path_index_query.rs)::`ingest_malformed_xml_bails` |
| `IDX-061` | Packages::glob picks highest version per name, not last-walked repo row | [library_match.rs](library_match.rs)::`packages_glob_picks_highest_version_per_name`, [search.rs](search.rs)::`search_libc6_yields_single_newest_row` |
| `IDX-062` | providers glob (library provides leg) attributes to newest package release | [library_match.rs](library_match.rs)::`providers_glob_attributes_soname_to_newest_release` |
| `IDX-064` | --from env fallback FLATROOT_ARG_FROM drives the search source | [search.rs](search.rs)::`search_from_env_drives_real_search` |
| `IDX-065` | --arch multi-value builds/consults a per-arch index for search/query | [search.rs](search.rs)::`search_arch_consults_per_arch_catalogue` |
| `IDX-066` | search uses the same canonical library form across all distro encodings | [library_match.rs](library_match.rs)::`library_glob_canonicalizes_each_encoding_to_one_form`, [search.rs](search.rs)::`search_library_canonical_form_across_families` |
| `IDX-067` | search `--type path --match all` renders only the common owner's pairs (live) | [cli_network.rs](cli_network.rs)::`search_path_match_all_keeps_only_common_owner` |
| `IDX-068` | search `--type library --match all` renders only the common owner's pairs (live) | [cli_network.rs](cli_network.rs)::`search_library_match_all_keeps_only_common_owner` |
| `IDX-069` | search `--type path --match any` renders every owner's pairs (live) | [cli_network.rs](cli_network.rs)::`search_path_match_any_keeps_all_owners` |
| `IDX-070` | search `--type path --match all` disjoint emits zero bytes (live) | [cli_network.rs](cli_network.rs)::`search_path_match_all_disjoint_is_empty` |
| `IDX-071` | search `--type path --match all` single pattern is identity (live) | [cli_network.rs](cli_network.rs)::`search_path_match_all_single_pattern_identity` |
| `IDX-073` | Concurrent index population: second process blocks on flock then TOCTOU fast-path | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`concurrent_index_population_one_populates_other_waits` |
| `IDX-074` | Stale .db rebuild removes WAL/SHM AND sibling .pathidx before repopulating | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`stale_db_rebuild_sweeps_wal_shm_and_pathidx_siblings` |
| `IDX-075` | DB cache TTL and listing-body TTL are independent 3600s windows | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`db_cache_and_listing_body_ttls_are_independent` |
| `IDX-076` | ArchContext::open (async) vs open_blocking (sync) identical env + framing | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`open_and_open_blocking_share_one_cached_index` |
| `IDX-077` | Per-arch index identity: search/query builds SEPARATE .db per arch | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`per_arch_index_builds_distinct_db_files` |
| `IDX-078` | search `--type path --match all` on apk refused (live) | [cli_network.rs](cli_network.rs)::`search_path_match_all_on_apk_refused` |
| `IDX-079` | library `--match all` on apk is allowed, not refused (live) | [cli_network.rs](cli_network.rs)::`search_library_match_all_on_apk_not_refused` |

## Distributions, remotes, mirrors & releases (63)

| ID | Scenario | Covering test |
|---|---|---|
| `DIST-001` | remote list (plain) emits all 10 distros with format/name/status | [release_list.rs](release_list.rs)::`remote_list_plain_per_entry_shape_in_registry_order` |
| `DIST-002` | remote list --format json emits bijective JSON array | [release_list.rs](release_list.rs)::`remote_list_json_is_array_of_format_name_status_in_order` |
| `DIST-003` | remote list works offline (no mirror contacted) | [release_list.rs](release_list.rs)::`remote_list_plain_per_entry_shape_in_registry_order` |
| `DIST-004` | remote list ignores --from when supplied (no error) | [release_list.rs](release_list.rs)::`remote_list_ignores_from_when_supplied` |
| `DIST-005` | remote list honours FLATROOT_ARG_REMOTE_LIST_FORMAT env | [release_list.rs](release_list.rs)::`remote_list_honours_format_env_var` |
| `DIST-006` | remote list rejects unknown --format at parse time | [release_list.rs](release_list.rs)::`remote_list_rejects_unknown_format_at_parse` |
| `DIST-007` | release list debian (Walk DebCodename) returns Codename/Mirror live | [release_list.rs](release_list.rs)::`release_list_debian_codenames_skip_aliases_and_host_only_mirror` |
| `DIST-008` | release list ubuntu over archive + old-releases | [release_list.rs](release_list.rs)::`release_list_ubuntu_codenames_sorted_with_normalized_keys` |
| `DIST-009` | release list alpine (Walk ApkVersion) Release/Type/Mirror stable+edge | [release_list.rs](release_list.rs)::`release_list_alpine_numeric_sort_and_release_name_header` |
| `DIST-015` | release list centos (Static) 4-column Codename/Version/Suite/Mirror | [release_list.rs](release_list.rs)::`release_list_centos_json_four_column_header_three_rows` |
| `DIST-020` | release list honours FLATROOT_ARG_FROM env | [release_list.rs](release_list.rs)::`release_list_honours_from_env_var` |
| `DIST-021` | release list --format json yields per-distro field-named objects | [release_list.rs](release_list.rs)::`release_list_json_objects_have_normalized_btreemap_keys` |
| `DIST-022` | release list plain drops cells lacking a header column | [mirror_snapshot.rs](mirror_snapshot.rs)::`release_row_drops_cells_lacking_a_header_column` |
| `DIST-023` | release list header_normalize maps 'Release'→'name', lowercases others | [release_list.rs](release_list.rs)::`release_list_ubuntu_codenames_sorted_with_normalized_keys`, [release_list.rs](release_list.rs)::`release_list_json_objects_have_normalized_btreemap_keys` |
| `DIST-024` | release list debian walk bails when every mirror returns empty | [mirror_snapshot.rs](mirror_snapshot.rs)::`deb_codename_walk_bails_when_every_mirror_empty` |
| `DIST-025` | release list RhelMajor bails when no numeric major dirs | [mirror_snapshot.rs](mirror_snapshot.rs)::`rhel_major_walk_numeric_only_first_mirror_and_bail_when_none` |
| `DIST-026` | release list deb walk propagates HEAD failure (verbose warns) | [mirror_snapshot.rs](mirror_snapshot.rs)::`deb_walk_propagates_head_failure_during_validation` |
| `DIST-027` | release list apk validation skips releases missing main or community index | [mirror_snapshot.rs](mirror_snapshot.rs)::`apk_version_walk_drops_release_missing_community_index` |
| `DIST-028` | release list empty result emits zero bytes (plain) vs [] (json) | [release_list.rs](release_list.rs)::`release_list_static_json_array_vs_plain_zero_bytes_for_empty_via_remote` |
| `DIST-029` | --from parse requires colon for source-building commands | [distro_source.rs](distro_source.rs)::`from_selector_requires_colon` |
| `DIST-030` | DistroRegistry::source rejects unknown distro prefix with 'Unknown remote' | [distro_source.rs](distro_source.rs)::`unknown_distro_prefix_rejected_with_supported_list` |
| `DIST-031` | source rejects arch the distro does not support | [distro_source.rs](distro_source.rs)::`arch_unsupported_by_distro_rejected` |
| `DIST-032` | arch source rejects malformed @date | [distro_source.rs](distro_source.rs)::`arch_malformed_date_rejected` |
| `DIST-033` | arch @date selects snapshot mirror archive.archlinux.org/repos/Y/M/D | [distro_source.rs](distro_source.rs)::`arch_at_date_routes_to_archive_repos_y_m_d` |
| `DIST-035` | cachyos source rejects any release other than 'rolling' | [distro_source.rs](distro_source.rs)::`cachyos_rejects_non_rolling_release` |
| `DIST-037` | alma source rejects release outside {8,9} | [distro_source.rs](distro_source.rs)::`alma_rejects_release_outside_8_9` |
| `DIST-038` | rocky source rejects release outside {8,9} | [distro_source.rs](distro_source.rs)::`rocky_rejects_release_outside_8_9` |
| `DIST-039` | centos per-release host+layout (7/8 vault, stream9 stream host) | [distro_source.rs](distro_source.rs)::`centos_per_release_host_and_layout_across_arches`, [remote_centos.rs](remote_centos.rs)::`search_bash_stream9_aarch64` |
| `DIST-040` | centos rejects unknown release | [distro_source.rs](distro_source.rs)::`centos_rejects_unknown_release` |
| `DIST-041` | fedora rawhide source: single primary mirror + development/rawhide repo | [distro_source.rs](distro_source.rs)::`fedora_rawhide_single_mirror_one_repo_across_arches`, [remote_fedora.rs](remote_fedora.rs)::`search_bash_fedora42_aarch64` |
| `DIST-042` | fedora numbered release uses primary + archive fallback | [distro_source.rs](distro_source.rs)::`fedora_numbered_uses_primary_then_archive_fallback` |
| `DIST-043` | opensuse tumbleweed vs leap, native vs ports path selection | [distro_source.rs](distro_source.rs)::`opensuse_native_vs_ports_path_selection`, [remote_opensuse.rs](remote_opensuse.rs)::`search_bash_tumbleweed_aarch64_ports` |
| `DIST-044` | debian @date selects snapshot.debian.org with stamp + T000000Z | [distro_source.rs](distro_source.rs)::`debian_at_date_routes_to_single_snapshot_mirror`, [distro_source.rs](distro_source.rs)::`debian_declares_per_arch_and_all_contents_listings`, [distro_source.rs](distro_source.rs)::`debian_at_date_snapshot_across_arches` |
| `DIST-046` | ubuntu @date snapshot routes archive AND security to one snapshot | [distro_source.rs](distro_source.rs)::`ubuntu_at_date_routes_archive_and_security_to_one_snapshot`, [remote_ubuntu.rs](remote_ubuntu.rs)::`search_bash_noble_at_date_snapshot` |
| `DIST-047` | ubuntu unpinned builds suite+updates+security × main/universe | [distro_source.rs](distro_source.rs)::`ubuntu_unpinned_builds_six_repos_contents_only_on_base_main`, [distro_source.rs](distro_source.rs)::`ubuntu_unpinned_archive_falls_back_to_old_releases` |
| `DIST-049` | alpine i686 host token x86 and i686 both → Arch::I686 → alpine 'x86' | [distro_source.rs](distro_source.rs)::`alpine_x86_and_i686_both_resolve_to_native_x86`, [remote_alpine.rs](remote_alpine.rs)::`cli_install_busybox_v3_21_x86`, [remote_alpine.rs](remote_alpine.rs)::`search_busybox_v3_21_i686_token` |
| `DIST-050` | --arch unknown uname token rejected before any source work | [distro_source.rs](distro_source.rs)::`unknown_arch_token_rejected_before_source_work` |
| `DIST-051` | --arch default is host architecture when omitted | [distro_source.rs](distro_source.rs)::`arch_default_equals_from_host_as_uname` |
| `DIST-052` | ListMirror fetch falls over to next mirror on first failure | [mirror_snapshot.rs](mirror_snapshot.rs)::`list_mirror_fetch_falls_over_to_second_on_first_failure` |
| `DIST-053` | ListMirror fetch bails with combined error when all fail | [mirror_snapshot.rs](mirror_snapshot.rs)::`list_mirror_fetch_bails_combined_error_when_all_fail` |
| `DIST-054` | ListMirror select returns first probe-true mirror, bails when none | [mirror_snapshot.rs](mirror_snapshot.rs)::`list_mirror_select_returns_first_probe_true_else_bails`, [mirror_snapshot.rs](mirror_snapshot.rs)::`list_mirror_select_propagates_probe_error` |
| `DIST-055` | ListMirror fetch_all/fetch_listings: any failure aborts (aggregate) | [mirror_snapshot.rs](mirror_snapshot.rs)::`list_mirror_fetch_all_visits_every_mirror_and_aborts_on_failure`, [mirror_snapshot.rs](mirror_snapshot.rs)::`list_mirror_fetch_listings_aborts_on_any_failure` |
| `DIST-056` | HttpMirror probe maps 404/Not Found to false, other errors to Err | [mirror_snapshot.rs](mirror_snapshot.rs)::`probe_maps_404_to_false_and_other_errors_to_err` |
| `DIST-057` | HttpMirror exists uses HEAD vs fetch GET | [mirror_snapshot.rs](mirror_snapshot.rs)::`exists_uses_head_while_fetch_uses_get`, [mirror_snapshot.rs](mirror_snapshot.rs)::`exists_missing_is_false_and_error_carries_context` |
| `DIST-058` | HttpMirror fetch_listing rejects non-UTF-8 listing bodies | [mirror_snapshot.rs](mirror_snapshot.rs)::`fetch_listing_rejects_non_utf8_body` |
| `DIST-059` | HttpMirror url_join normalizes the slash between base and rel | [mirror_snapshot.rs](mirror_snapshot.rs)::`url_join_normalizes_to_exactly_one_slash` |
| `DIST-060` | listing_parse extracts href dir names, applies family accept filter, dedups | [mirror_snapshot.rs](mirror_snapshot.rs)::`deb_codename_walk_filters_aliases_dedups_and_sorts` |
| `DIST-061` | WalkStrategy.accept rejects Debian lifecycle aliases | [mirror_snapshot.rs](mirror_snapshot.rs)::`deb_codename_walk_filters_aliases_dedups_and_sorts` |
| `DIST-062` | ApkVersion sort_key orders versions numerically not lexically | [mirror_snapshot.rs](mirror_snapshot.rs)::`apk_version_walk_sorts_numerically_not_lexically`, [release_list.rs](release_list.rs)::`release_list_alpine_numeric_sort_and_release_name_header` |
| `DIST-063` | RhelMajor scrape uses first mirror; APK scrape uses first mirror | [mirror_snapshot.rs](mirror_snapshot.rs)::`rhel_major_walk_numeric_only_first_mirror_and_bail_when_none` |
| `DIST-064` | mirror_label strips https:// and keeps only host | [mirror_snapshot.rs](mirror_snapshot.rs)::`deb_walk_mirror_label_is_the_host_only`, [release_list.rs](release_list.rs)::`release_list_debian_codenames_skip_aliases_and_host_only_mirror` |
| `DIST-065` | release list runs in spawn_blocking (sync collect under async runtime) | [release_list.rs](release_list.rs)::`release_list_debian_codenames_skip_aliases_and_host_only_mirror`, [release_list.rs](release_list.rs)::`release_list_runs_under_spawn_blocking_identical_output` |
| `DIST-066` | release list honours --http-retries / env for the scrape client | [release_list.rs](release_list.rs)::`release_list_http_retries_flag_accepted_for_static_distro` |
| `DIST-067` | Static release listing never issues a network request despite client build | [release_list.rs](release_list.rs)::`release_list_http_retries_flag_accepted_for_static_distro`, [release_list.rs](release_list.rs)::`release_list_static_distros_succeed_without_network` |
| `DIST-068` | Walk release listing requires network and fails when mirror unreachable | [mirror_snapshot.rs](mirror_snapshot.rs)::`deb_walk_fails_when_listing_unreachable`, [release_list.rs](release_list.rs)::`release_list_debian_codenames_skip_aliases_and_host_only_mirror` |
| `DIST-069` | rpm-numeric walk parses non-numeric dir names as skipped (continue) | [mirror_snapshot.rs](mirror_snapshot.rs)::`rpm_numeric_walk_skips_non_numeric_and_appends_extras`, [release_list.rs](release_list.rs)::`release_list_fedora_appends_rawhide_skips_non_numeric` |
| `DIST-070` | Per-distro from_syntax strings surface verbatim in remote list | [release_list.rs](release_list.rs)::`remote_list_plain_per_entry_shape_in_registry_order`, [release_list.rs](release_list.rs)::`remote_list_json_is_array_of_format_name_status_in_order` |
| `DIST-072` | download_url panics on unprefixed filename (index-population invariant) | [distro_source.rs](distro_source.rs)::`download_url_errors_on_unprefixed_filename` |
| `DIST-074` | fedora/opensuse accept arbitrary release strings, deferring failure to fetch | [distro_source.rs](distro_source.rs)::`fedora_arbitrary_release_builds_without_bail`, [distro_source.rs](distro_source.rs)::`opensuse_arbitrary_release_builds_without_bail` |
| `DIST-075` | debian/ubuntu/alpine accept arbitrary release/codename (no build-time bail) | [distro_source.rs](distro_source.rs)::`debian_ubuntu_alpine_accept_arbitrary_release_no_build_bail` |
| `DIST-077` | @date source axis across install/search/query/analyze/release | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`dated_source_axis_honoured_and_cache_namespaced` |
| `DIST-078` | download_url base-trim/rel-trim slash normalization joins exactly one slash | [distro_source.rs](distro_source.rs)::`download_url_joins_with_exactly_one_slash` |
| `DIST-079` | release list spawn_blocking; Static distros ZERO network despite client build | [release_list.rs](release_list.rs)::`release_source_static_does_no_network_walk_needs_mirrors` |

## Package formats: parse & extract (51)

| ID | Scenario | Covering test |
|---|---|---|
| `PKG-002` | Deb stanza parse: continuation lines folded, blank line ends block | [pkg_parse.rs](pkg_parse.rs)::`deb_catalogue_parse_yields_well_formed_packages` |
| `PKG-003` | Deb stanza build: non-integer Size is a hard error | [pkg_parse.rs](pkg_parse.rs)::`deb_size_parses_as_integer` |
| `PKG-004` | Deb dependency parse: comma groups, alternatives, constraints, arch qualifier stripped | [pkg_parse.rs](pkg_parse.rs)::`deb_dependency_parse_groups_alternatives_constraints_and_arch_qualifier` |
| `PKG-005` | Deb provides parse: `= version` constraint normalized | [pkg_parse.rs](pkg_parse.rs)::`deb_provides_version_is_normalized_without_operator` |
| `PKG-006` | Deb priority ordinal mapping steers provider tie-breaks | [pkg_parse.rs](pkg_parse.rs)::`deb_priority_ordinal_steers_provider_order` |
| `PKG-008` | Deb Contents ingest skips header/preamble/blank and slash-less paths | [pkg_parse.rs](pkg_parse.rs)::`deb_contents_ingest_resolves_library_owner` |
| `PKG-009` | Deb Contents ingest: multi-package line attributes every owner | [pkg_parse.rs](pkg_parse.rs)::`deb_contents_ingest_resolves_library_owner` |
| `PKG-010` | Deb Contents ingest tolerates non-UTF-8 path bytes (lossy, stream continues) | [pkg_parse.rs](pkg_parse.rs)::`deb_contents_ingest_tolerates_non_utf8_suite` |
| `PKG-011` | Deb index_fetch rejects non-Deb repository layout | [pkg_parse.rs](pkg_parse.rs)::`deb_index_fetch_rejects_non_deb_layout` |
| `PKG-013` | Deb extract: codec auto-detected from data.tar member name | [pkg_extract.rs](pkg_extract.rs)::`deb_extract_auto_detects_data_tar_codec` |
| `PKG-014` | Deb extract: missing data.tar.* is a hard error | [pkg_extract.rs](pkg_extract.rs)::`deb_extract_missing_data_tar_is_hard_error` |
| `PKG-015` | Deb extract: unopenable archive file errors with context | [pkg_extract.rs](pkg_extract.rs)::`deb_extract_unopenable_archive_errors_with_context` |
| `PKG-017` | RPM repomd_href: locates href for named data type, errors if absent | [pkg_parse.rs](pkg_parse.rs)::`rpm_repomd_resolution_and_metadata_codec_decode_across_distros` |
| `PKG-018` | RPM metadata codec by href suffix; invalid UTF-8 errors | [pkg_parse.rs](pkg_parse.rs)::`rpm_repomd_resolution_and_metadata_codec_decode_across_distros` |
| `PKG-019` | RPM primary parse: arch/noarch kept, foreign-arch dropped | [pkg_parse.rs](pkg_parse.rs)::`rpm_primary_parse_keeps_target_and_noarch_drops_foreign` |
| `PKG-020` | RPM primary parse: version assembled with epoch only when non-zero | [pkg_parse.rs](pkg_parse.rs)::`rpm_version_carries_epoch_only_when_nonzero` |
| `PKG-021` | RPM requires parse: rpmlib()/config()/rtld()/build-only skipped | [pkg_parse.rs](pkg_parse.rs)::`rpm_requires_skip_internal_pseudo_deps` |
| `PKG-022` | RPM requires parse: flag→operator mapping (EQ/GE/GT/LE/LT, default GE) | [pkg_parse.rs](pkg_parse.rs)::`rpm_requires_use_dpkg_spelled_operators` |
| `PKG-023` | RPM rich-dep parse: conditional kept as AST, non-conditional flattened | [pkg_parse.rs](pkg_parse.rs)::`rpm_rich_deps_defer_conditionals` |
| `PKG-024` | RPM rich-dep parse errors: empty expr, unknown op, flatten conditional | [pkg_parse.rs](pkg_parse.rs)::`rpm_rich_deps_defer_conditionals` |
| `PKG-025` | RPM rich-dep leaf: arch qualifier and version operator split | [pkg_parse.rs](pkg_parse.rs)::`rpm_rich_dep_leaf_strips_arch_qualifier_from_names` |
| `PKG-027` | RPM extract: payload codec by magic (xz/gzip/zstd), unknown errors | [pkg_extract.rs](pkg_extract.rs)::`rpm_extract_decompresses_each_payload_codec_and_rejects_unknown` |
| `PKG-028` | RPM extract: no payload offset → 'No compressed payload found' | [pkg_extract.rs](pkg_extract.rs)::`rpm_extract_no_payload_offset_errors_with_path` |
| `PKG-029` | RPM cpio extract: newc magic validated, files with exec bit, symlinks | [pkg_extract.rs](pkg_extract.rs)::`rpm_cpio_extract_newc_files_exec_bit_and_symlink` |
| `PKG-030` | RPM cpio extract: directory→symlink replacement for merged-/usr | [pkg_extract.rs](pkg_extract.rs)::`rpm_cpio_extract_replaces_empty_dir_with_symlink_for_merged_usr` |
| `PKG-031` | RPM cpio extract: device nodes silently skipped, symlink fail warns | [pkg_extract.rs](pkg_extract.rs)::`rpm_cpio_extract_skips_device_nodes_and_symlink_failure_warns` |
| `PKG-032` | RPM cpio extract: malformed hex header fields are hard errors | [pkg_extract.rs](pkg_extract.rs)::`rpm_cpio_extract_malformed_hex_header_is_hard_error` |
| `PKG-033` | RPM scripts_stage: POSTIN saved; Lua interpreter → postinst.lua | [pkg_extract.rs](pkg_extract.rs)::`rpm_scripts_stage_shell_lua_and_empty` |
| `PKG-034` | RPM index_fetch rejects non-Rpm repository layout | [pkg_parse.rs](pkg_parse.rs)::`rpm_index_fetch_rejects_non_rpm_layout` |
| `PKG-035` | RPM size attribute parse: non-integer <size package=...> hard error | [pkg_parse.rs](pkg_parse.rs)::`rpm_size_attribute_parses_as_integer` |
| `PKG-036` | RPM primary parse: malformed primary XML is a hard error | [pkg_parse.rs](pkg_parse.rs)::`rpm_repomd_resolution_and_metadata_codec_decode_across_distros` |
| `PKG-039` | APKINDEX parse: P/V/T/S/D/p/i/C fields, block boundary, trailing flush | [pkg_parse.rs](pkg_parse.rs)::`apk_index_parse_yields_well_formed_packages` |
| `PKG-040` | APK deps parse: version ops incl ~/~=, negative deps skipped | [pkg_parse.rs](pkg_parse.rs)::`apk_deps_carry_operators_and_exclude_negatives` |
| `PKG-041` | APK provides parse: name=version capabilities captured | [pkg_parse.rs](pkg_parse.rs)::`apk_provides_capture_versioned_capabilities` |
| `PKG-042` | APK install-if parse: companion names with constraints stripped | [pkg_parse.rs](pkg_parse.rs)::`apk_install_if_stores_bare_companion_names` |
| `PKG-044` | APK extract: .PKGINFO and other control files skipped, only install scripts | [pkg_extract.rs](pkg_extract.rs)::`apk_extract_stages_only_install_scripts` |
| `PKG-045` | APK extract: unreadable archive errors with context | [pkg_extract.rs](pkg_extract.rs)::`apk_extract_unreadable_archive_errors_with_context` |
| `PKG-046` | APK index_fetch rejects non-Apk repository layout | [pkg_parse.rs](pkg_parse.rs)::`apk_index_fetch_rejects_non_apk_layout` |
| `PKG-048` | Pacman .db codec detection by content magic (zstd vs gzip fallback) | [pkg_parse.rs](pkg_parse.rs)::`pacman_db_decodes_via_content_magic` |
| `PKG-049` | Pacman desc parse: %FIELD% headers; missing NAME/VERSION/FILENAME→no entry | [pkg_parse.rs](pkg_parse.rs)::`pacman_desc_parse_yields_complete_records` |
| `PKG-050` | Pacman dep parse: version ops split, OPTDEPENDS description stripped | [pkg_parse.rs](pkg_parse.rs)::`pacman_optdepends_become_suggests_without_description` |
| `PKG-052` | Pacman pkg_name_from_dir: strip pkgver-pkgrel, decline malformed labels | [pkg_parse.rs](pkg_parse.rs)::`pacman_pkg_name_from_dir_resolves_bare_owner` |
| `PKG-054` | Pacman extract: existing non-dir dest removed before unpack | [pkg_extract.rs](pkg_extract.rs)::`pacman_extract_replaces_plain_file_preserves_directory` |
| `PKG-055` | Pacman index_fetch rejects non-Pacman repository layout | [pkg_parse.rs](pkg_parse.rs)::`pacman_index_fetch_rejects_non_pacman_layout` |
| `PKG-057` | Tar::extract: later entries win, obstructing dests cleared, malformed skipped | [pkg_codec.rs](pkg_codec.rs)::`tar_extract_last_writer_wins_and_survives_corrupt_entry` |
| `PKG-058` | Tar::extract_compressed: codec inferred from member name then extracted | [pkg_codec.rs](pkg_codec.rs)::`tar_extract_compressed_infers_codec_from_member_name` |
| `PKG-059` | Codec::reader/bytes: all five codecs decode consistently | [pkg_codec.rs](pkg_codec.rs)::`codec_reader_and_bytes_round_trip_all_five` |
| `PKG-060` | Codec::from_magic ambiguity: only zstd recognized, else assumed gzip | [pkg_codec.rs](pkg_codec.rs)::`codec_from_magic_recognises_zstd_else_assumes_gzip` |
| `PKG-062` | Merged-/usr extraction ordering hazard observable end-to-end | [pkg_parse.rs](pkg_parse.rs)::`merged_usr_layout_is_materialized_end_to_end` |
| `PKG-063` | RPM filelists ingest into path index alongside primary parse | [pkg_parse.rs](pkg_parse.rs)::`rpm_filelists_resolve_soname_via_path_index` |
| `PKG-064` | Pacman extract: unopenable archive errors (File::open / zstd::Decoder) | [pkg_extract.rs](pkg_extract.rs)::`pacman_extract_unopenable_archive_aborts` |

## Post-install: ldconfig, scripts, hooks, stubs (61)

| ID | Scenario | Covering test |
|---|---|---|
| `POST-001` | Full default post-install runs all three phases in order | [postinstall_scripts.rs](postinstall_scripts.rs)::`default_postinstall_runs_three_phases_in_fixed_order` |
| `POST-002` | Empty phase list is an immediate no-op before the sandbox gate | [postinstall_scripts.rs](postinstall_scripts.rs)::`empty_phase_list_is_immediate_noop_before_sandbox_gate` |
| `POST-005` | Selecting a single phase runs only that phase | [postinstall_scripts.rs](postinstall_scripts.rs)::`single_phase_scripts_runs_only_scripts_and_is_silent_on_success` |
| `POST-006` | --postinstall accepts comma subset and preserves fixed execution order | [postinstall_scripts.rs](postinstall_scripts.rs)::`comma_subset_preserves_fixed_execution_order` |
| `POST-007` | FLATROOT_ARG_INSTALL_POSTINSTALL env var supplies phase list | [postinstall_scripts.rs](postinstall_scripts.rs)::`env_var_supplies_phase_list_none` |
| `POST-009` | Post-install aborts with remediation when userns unavailable | [sandbox_smoke.rs](sandbox_smoke.rs)::`postinstall_aborts_with_remediation_when_userns_unavailable` |
| `POST-010` | Sandbox availability probe succeeds on a permissive host | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_available_succeeds_on_permissive_host` |
| `POST-012` | Sandbox::run maps the calling user to uid/gid 0 inside the namespace | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_against_real_rootfs_covers_identity_codes_dev_and_capture` |
| `POST-013` | Sandbox::run with an empty command bails | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_empty_command_bails_before_fork` |
| `POST-014` | Sandbox::run reports a non-zero child exit code without aborting | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_against_real_rootfs_covers_identity_codes_dev_and_capture` |
| `POST-015` | Sandbox::run translates a signalled child to 128+signal | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_against_real_rootfs_covers_identity_codes_dev_and_capture` |
| `POST-016` | Sandbox::run reports exec-failure exit code 127 for missing command | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_missing_command_yields_exit_127` |
| `POST-017` | Sandbox::run reports exit 126 when mount setup or unshare fails | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_mount_setup_failure_yields_exit_126` |
| `POST-018` | Sandbox provides /dev tmpfs with device nodes and fd symlinks | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_against_real_rootfs_covers_identity_codes_dev_and_capture` |
| `POST-019` | Sandbox::run captures stdout/stderr separately and clears env | [sandbox_exec.rs](sandbox_exec.rs)::`sandbox_run_against_real_rootfs_covers_identity_codes_dev_and_capture` |
| `POST-020` | ldconfig phase runs rootfs ldconfig, reports success on glibc | [postinstall_hooks.rs](postinstall_hooks.rs)::`ldconfig_phase_populates_cache_on_glibc_distros` |
| `POST-021` | ldconfig phase auto-skips on a musl (Alpine) tree | [postinstall_hooks.rs](postinstall_hooks.rs)::`ldconfig_phase_auto_skips_on_musl_alpine` |
| `POST-022` | ldconfig non-zero exit reported but does not fail the build | [postinstall_hooks.rs](postinstall_hooks.rs)::`ldconfig_nonzero_exit_is_reported_but_still_ok` |
| `POST-023` | Scripts phase is a no-op when .flatroot/scripts is absent | [postinstall_scripts.rs](postinstall_scripts.rs)::`scripts_phase_noop_when_scripts_dir_absent` |
| `POST-024` | Scripts phase skips replay when interpreter absent from rootfs | [postinstall_scripts.rs](postinstall_scripts.rs)::`scripts_phase_skips_when_interpreter_absent` |
| `POST-025` | Scripts phase is a no-op when no staged package eligible | [postinstall_scripts.rs](postinstall_scripts.rs)::`scripts_phase_noop_when_no_eligible_package` |
| `POST-026` | ScriptEntry parses package name and arch from dir name, defaults arch | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure` |
| `POST-027` | Staged scripts run in sorted (deterministic) order | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure` |
| `POST-028` | Debian-family postinst replay: 'configure' + DPKG_MAINTSCRIPT_* env | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure`, [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_blackbox_scripts_replay_cleans_up_temp_script` |
| `POST-029` | Debian flavour excludes libc6 from postinst replay | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure` |
| `POST-030` | Debian flavour skips Lua-form postinst it cannot interpret | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure` |
| `POST-031` | Debian flavour eligibility requires a plain postinst to exist | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure` |
| `POST-032` | Debian stubs: noop stand-ins for privileged/system-management commands | [postinstall_stubs.rs](postinstall_stubs.rs)::`debian_noop_stubs_written_0755_under_flatroot_bin` |
| `POST-033` | Debian dpkg stub answers --compare-versions and --status | [postinstall_stubs.rs](postinstall_stubs.rs)::`debian_dpkg_stub_answers_compare_versions_and_status` |
| `POST-034` | Debian update-alternatives stub actually creates symlinks | [postinstall_stubs.rs](postinstall_stubs.rs)::`debian_update_alternatives_stub_creates_primary_and_slave_symlinks` |
| `POST-035` | Debian debconf confmodule/frontend seeded so scripts don't hang | [postinstall_stubs.rs](postinstall_stubs.rs)::`debian_debconf_confmodule_and_frontend_seeded` |
| `POST-036` | Debian /etc/shells seeded only if missing; add-shell appends idempotently | [postinstall_stubs.rs](postinstall_stubs.rs)::`debian_etc_shells_seeded_only_if_missing_add_shell_guarded` |
| `POST-037` | Arch-family install replay: only post_install-bearing files run via bash wrapper | [postinstall_scripts.rs](postinstall_scripts.rs)::`arch_and_cachyos_replay_run_only_post_install_bearing_packages`, [postinstall_scripts.rs](postinstall_scripts.rs)::`arch_blackbox_scripts_reaches_arch_finish_marker` |
| `POST-038` | Arch eligibility false when install missing/unreadable/lacks post_install | [postinstall_scripts.rs](postinstall_scripts.rs)::`arch_and_cachyos_replay_run_only_post_install_bearing_packages` |
| `POST-039` | Arch stubs use noop_keep and a vercmp stub returning equal | [postinstall_stubs.rs](postinstall_stubs.rs)::`arch_stubs_use_noop_keep_and_vercmp_equal_without_cache_regen` |
| `POST-040` | CachyOS post-install is identical to Arch by delegation | [postinstall_scripts.rs](postinstall_scripts.rs)::`arch_and_cachyos_replay_run_only_post_install_bearing_packages`, [postinstall_scripts.rs](postinstall_scripts.rs)::`cachyos_blackbox_scripts_reaches_arch_finish_marker` |
| `POST-041` | Alpine post-install replay: runs with sh and package name as argv[1] | [postinstall_scripts.rs](postinstall_scripts.rs)::`alpine_replay_passes_package_name_as_argv1`, [postinstall_scripts.rs](postinstall_scripts.rs)::`alpine_blackbox_scripts_reaches_alpine_finish_marker` |
| `POST-042` | Alpine stage surfaces a real read error of an existing post-install script | [postinstall_scripts.rs](postinstall_scripts.rs)::`alpine_stage_propagates_read_error_of_existing_script` |
| `POST-043` | Alpine stubs neutralize rc/apk commands via noop_keep | [postinstall_stubs.rs](postinstall_stubs.rs)::`alpine_stubs_neutralize_rc_and_apk_via_noop_keep` |
| `POST-044` | stage() returning None declines a package quietly and advances the bar | [../src/postinstall/script.rs](../src/postinstall/script.rs)::`stage_returning_none_advances_and_skips_without_cleanup` |
| `POST-045` | A failing postinst reported via StepReport but replay continues | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure` |
| `POST-046` | StepReport stays silent on a successful step | [postinstall_scripts.rs](postinstall_scripts.rs)::`single_phase_scripts_runs_only_scripts_and_is_silent_on_success` |
| `POST-047` | StepReport reports a step that could not be attempted (Err) | [../src/postinstall/step_report.rs](../src/postinstall/step_report.rs)::`could_not_be_attempted_reports_the_error_and_keeps_going` |
| `POST-048` | Hooks phase rebuilds every applicable cache, silent/best-effort otherwise | [postinstall_hooks.rs](postinstall_hooks.rs)::`hooks_run_present_builders_in_table_order_skipping_absent` |
| `POST-049` | Hook skipped when its builder binary is not present | [postinstall_hooks.rs](postinstall_hooks.rs)::`hooks_run_present_builders_in_table_order_skipping_absent` |
| `POST-050` | Icon-cache hook skipped unless hicolor index.theme present | [postinstall_hooks.rs](postinstall_hooks.rs)::`icon_cache_hook_gated_on_hicolor_index` |
| `POST-051` | locale-gen hook seeds /etc/locale.gen when missing then runs | [postinstall_hooks.rs](postinstall_hooks.rs)::`locale_gen_hook_seeds_when_missing_and_leaves_existing` |
| `POST-052` | gio-modules hook pointed at resolved gio/modules dir or flat default | [postinstall_hooks.rs](postinstall_hooks.rs)::`gio_modules_hook_uses_resolved_dir_else_flat_default` |
| `POST-053` | Arch-class-suffixed cache builders found via basename preference list | [postinstall_hooks.rs](postinstall_hooks.rs)::`arch_suffixed_cache_builder_located_first_listed_wins` |
| `POST-054` | Both CA-trust families listed; whichever the rootfs ships is run | [postinstall_hooks.rs](postinstall_hooks.rs)::`ca_trust_both_families_run_when_present` |
| `POST-055` | A hook builder exiting non-zero is reported but doesn't stop remaining | [postinstall_hooks.rs](postinstall_hooks.rs)::`hook_nonzero_exit_does_not_stop_remaining` |
| `POST-056` | binary_find prefers usr/* and probes lib subdirs only when given | [postinstall_hooks.rs](postinstall_hooks.rs)::`hooks_run_present_builders_in_table_order_skipping_absent` |
| `POST-057` | Stubs land under .flatroot/bin which is first on the script PATH | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure`, [postinstall_stubs.rs](postinstall_stubs.rs)::`debian_noop_stubs_written_0755_under_flatroot_bin` |
| `POST-058` | noop_keep / script_keep / seed_keep never overwrite existing tailored stub | [postinstall_stubs.rs](postinstall_stubs.rs)::`keep_variants_never_overwrite_existing_tailored_stub` |
| `POST-059` | Per-package cleanup removes staged temp scripts after each replay | [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_replay_parses_orders_filters_and_continues_on_failure`, [postinstall_scripts.rs](postinstall_scripts.rs)::`debian_blackbox_scripts_replay_cleans_up_temp_script`, [postinstall_scripts.rs](postinstall_scripts.rs)::`arch_and_cachyos_replay_run_only_post_install_bearing_packages`, [postinstall_scripts.rs](postinstall_scripts.rs)::`alpine_replay_passes_package_name_as_argv1` |
| `POST-060` | Post-install uses primary arch's uname as script-entry default arch | [postinstall_scripts.rs](postinstall_scripts.rs)::`multiarch_default_postinstall_runs_once_after_all_archs` |
| `POST-061` | One Sandbox session reused across all selected phases | [postinstall_scripts.rs](postinstall_scripts.rs)::`default_postinstall_runs_three_phases_in_fixed_order` |
| `POST-062` | RPM-family distros replay scriptlets through the Debian flavour path | [postinstall_scripts.rs](postinstall_scripts.rs)::`rpm_family_replays_through_debian_flavour_path` |
| `POST-063` | Sandbox availability gate checked once up-front; --postinstall=none bypasses | [install_pipeline.rs](install_pipeline.rs)::`postinstall_none_bypasses_the_sandbox_gate` |
| `POST-064` | Sandbox session reused; ldconfig→scripts→hooks fixed order regardless of token order | [install_pipeline.rs](install_pipeline.rs)::`scrambled_postinstall_tokens_still_run_in_fixed_order` |
| `POST-065` | Postfix runs whenever extraction occurred (incl none) and aborts on chmod fail | [install_pipeline.rs](install_pipeline.rs)::`postfix_repairs_mode_zero_files_to_0644` |

## Export formats (43)

| ID | Scenario | Covering test |
|---|---|---|
| `EXP-001` | tar.gz export inferred from .tar.gz extension | [export_format.rs](export_format.rs)::`targz_inferred_from_extension` |
| `EXP-002` | SquashFS export inferred from .sqfs extension | [export_format.rs](export_format.rs)::`sqfs_inferred_produces_valid_excluded_image` |
| `EXP-003` | SquashFS export inferred from .squashfs extension alias | [export_format.rs](export_format.rs)::`squashfs_alias_produces_valid_image` |
| `EXP-004` | DwarFS export inferred from .dwarfs extension | [export_format.rs](export_format.rs)::`dwarfs_inferred_produces_valid_excluded_image` |
| `EXP-006` | OCI export refused without --tag | [export_format.rs](export_format.rs)::`oci_without_tag_is_refused` |
| `EXP-007` | --tag ignored for non-OCI formats | [export_format.rs](export_format.rs)::`tag_is_ignored_for_non_oci_formats` |
| `EXP-008` | Explicit --format overrides extension inference | [export_format.rs](export_format.rs)::`explicit_format_overrides_foreign_extension` |
| `EXP-009` | Bare .tar extension is ambiguous and rejected without --format | [export_format.rs](export_format.rs)::`bare_tar_without_format_is_ambiguous` |
| `EXP-010` | Bare .tar with explicit --format tar succeeds | [export_format.rs](export_format.rs)::`bare_tar_with_explicit_tar_succeeds` |
| `EXP-011` | Bare .tar with explicit --format oci succeeds | [export_format.rs](export_format.rs)::`bare_tar_with_explicit_oci_succeeds` |
| `EXP-012` | Unknown/no extension rejected when format cannot be inferred | [export_format.rs](export_format.rs)::`unknown_extension_without_format_is_refused` |
| `EXP-013` | Output path with no file name component rejected | [export_format.rs](export_format.rs)::`output_with_no_file_name_is_refused` |
| `EXP-014` | Missing source directory rejected before any packaging | [export_format.rs](export_format.rs)::`missing_source_directory_is_refused` |
| `EXP-015` | Source that is a file (not a directory) rejected | [export_format.rs](export_format.rs)::`source_that_is_a_file_is_refused` |
| `EXP-016` | Source directory with no detectable arch binary rejected | [export_format.rs](export_format.rs)::`source_without_detectable_arch_is_refused` |
| `EXP-017` | DwarFS export fails clearly when mkdwarfs missing from PATH | [export_tool.rs](export_tool.rs)::`dwarfs_missing_tool_is_reported` |
| `EXP-018` | SquashFS export fails clearly when mksquashfs missing from PATH | [export_tool.rs](export_tool.rs)::`sqfs_missing_tool_is_reported` |
| `EXP-019` | DwarFS export reports tool non-zero exit as export failure | [export_tool.rs](export_tool.rs)::`dwarfs_tool_nonzero_exit_is_reported` |
| `EXP-020` | SquashFS export reports tool non-zero exit as export failure | [export_tool.rs](export_tool.rs)::`sqfs_tool_nonzero_exit_is_reported` |
| `EXP-026` | tar/oci exclusion is top-level-only (nested .flatroot kept) | [export_format.rs](export_format.rs)::`tar_and_oci_exclude_top_level_flatroot_only` |
| `EXP-029` | tar/oci entries are name-sorted for reproducible output | [export_format.rs](export_format.rs)::`tar_and_oci_entries_are_name_sorted_and_reproducible` |
| `EXP-030` | OCI diff_id is deterministic for an unchanged tree | [export_format.rs](export_format.rs)::`oci_diff_id_is_deterministic` |
| `EXP-031` | OCI archive uses plain USTAR (no GNU-sparse) for docker compat | [export_format.rs](export_format.rs)::`oci_layer_has_no_gnu_sparse_entries` |
| `EXP-032` | tar.gz uses plain USTAR (no GNU-sparse) | [export_format.rs](export_format.rs)::`targz_has_no_gnu_sparse_entries` |
| `EXP-033` | OCI platform metadata reflects detected arch (goarch + os linux) | [export_format.rs](export_format.rs)::`oci_platform_and_detected_arch_across_arches` |
| `EXP-034` | OCI export prints detected architecture (uname + goarch) to stderr | [export_format.rs](export_format.rs)::`oci_platform_and_detected_arch_across_arches` |
| `EXP-036` | OCI RepoTags echoes the exact --tag value | [export_format.rs](export_format.rs)::`oci_repo_tags_echo_exact_tag` |
| `EXP-037` | -t short alias works for --tag | [export_format.rs](export_format.rs)::`oci_short_tag_alias_works` |
| `EXP-038` | FLATROOT_ARG_EXPORT_FORMAT env fallback selects format | [export_format.rs](export_format.rs)::`env_export_format_selects_format` |
| `EXP-039` | FLATROOT_ARG_EXPORT_TAG env fallback supplies OCI tag | [export_format.rs](export_format.rs)::`env_export_tag_satisfies_oci_requirement` |
| `EXP-040` | Explicit --format beats FLATROOT_ARG_EXPORT_FORMAT env | [export_format.rs](export_format.rs)::`cli_format_beats_env_format` |
| `EXP-041` | Run order: source validation before format before tag before packaging | [export_format.rs](export_format.rs)::`source_validation_runs_before_format_and_tag` |
| `EXP-042` | OCI layer media type is tar+gzip; layer blob is gzip-compressed | [export_format.rs](export_format.rs)::`oci_layer_media_type_and_diff_id` |
| `EXP-043` | Empty rootfs directory export behavior | [export_format.rs](export_format.rs)::`source_without_detectable_arch_is_refused` |
| `EXP-044` | OCI tempdir staging is isolated and cleaned up | [export_format.rs](export_format.rs)::`oci_leaves_no_partial_output_on_failure` |
| `EXP-045` | Mutating export verb is silent on stdout (progress only on stderr) | [export_format.rs](export_format.rs)::`export_writes_nothing_to_stdout` |
| `EXP-046` | DwarFS/SquashFS reject non-UTF-8 output path | [export_tool.rs](export_tool.rs)::`dwarfs_sqfs_reject_non_utf8_output_path` |
| `EXP-047` | Tool-driven formats reject non-UTF-8 SOURCE path | [export_tool.rs](export_tool.rs)::`dwarfs_sqfs_reject_non_utf8_source_path` |
| `EXP-048` | tar/oci output File::create failure surfaced with context | [export_tool.rs](export_tool.rs)::`tar_and_oci_output_create_failure_is_contextualised` |
| `EXP-049` | DwarFS export produces a valid, exclusion-respecting image (real `mkdwarfs`) | [export_format.rs](export_format.rs)::`dwarfs_inferred_produces_valid_excluded_image` |
| `EXP-050` | SquashFS export produces a valid, exclusion-respecting image (real `mksquashfs`) | [export_format.rs](export_format.rs)::`sqfs_inferred_produces_valid_excluded_image` |
| `EXP-051` | OCI loadable end-to-end by docker and podman | [export_roundtrip.rs](export_roundtrip.rs)::`oci_export_roundtrips_gimp_debian_and_docker_loads`, [export_roundtrip.rs](export_roundtrip.rs)::`oci_export_loads_and_runs_under_podman` |
| `EXP-052` | OCI config diff_ids are uncompressed-content sha (engine reassembly) | [export_format.rs](export_format.rs)::`oci_layer_media_type_and_diff_id` |

## Analyze / trace — incl. `--match` seed resolution (52)

| ID | Scenario | Covering test |
|---|---|---|
| `ANL-001` | Default trace (both strategies) on single seed renders plain | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_default_plain_frames_single_seed_and_marks_target` |
| `ANL-002` | Strategy=declared only skips the linker pass | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_declared_skips_linker_pass` |
| `ANL-003` | Strategy=linker only skips the declared pass | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_linker_skips_declared_pass` |
| `ANL-004` | Strategy=both explicit equals default and unions reasons | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_both_explicit_equals_default_and_unions_reasons` |
| `ANL-005` | Both --no-deps and --no-linker rejected at run() | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_empty_strategy_rejected_before_fetch` |
| `ANL-006` | Empty --strategy= rejected at CLI before run() | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_empty_strategy_rejected_before_fetch` |
| `ANL-007` | Empty aggregate match exits 0 with zero stdout | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_empty_match_plain_zero_stdout_passes_never_run` |
| `ANL-011` | N-seed stderr framing truncates preview to first 5 with total | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_framing_truncates_preview_to_first_five` |
| `ANL-012` | N-seed framing (2..=5 seeds) lists all without truncation | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_framing_lists_all_seeds_for_two_to_five` |
| `ANL-013` | Single-seed framing uses name+version shape | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_default_plain_frames_single_seed_and_marks_target` |
| `ANL-015` | --type=library empty match exits 0 with zero stdout | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_library_empty_match_zero_stdout` |
| `ANL-016` | --type env fallback FLATROOT_ARG_ANALYZE_TRACE_TYPE | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_type_env_fallback_selects_library_mode` |
| `ANL-017` | --format env fallback FLATROOT_ARG_ANALYZE_TRACE_FORMAT | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_format_env_fallback_selects_json` |
| `ANL-018` | --strategy env fallback FLATROOT_ARG_ANALYZE_TRACE_STRATEGY | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_env_fallback_selects_declared_only` |
| `ANL-019` | --with env fallback FLATROOT_ARG_ANALYZE_TRACE_WITH | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_with_widens_declared_closure_via_flag_and_env` |
| `ANL-032` | Linker-only-reached package classified reason=linker (under-declaration) | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_both_explicit_equals_default_and_unions_reasons` |
| `ANL-033` | Declared-only package classified reason=declared | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_linker_skips_declared_pass`, [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_both_explicit_equals_default_and_unions_reasons` |
| `ANL-034` | Seed reason=target overrides declared/linker classification | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_both_explicit_equals_default_and_unions_reasons` |
| `ANL-035` | Unresolved soname raises trailing stderr warning with aggregate count | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_unresolved_warning_tracks_aggregate_count` |
| `ANL-036` | No unresolved sonames emits no trailing warning | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_unresolved_warning_tracks_aggregate_count` |
| `ANL-037` | Per-entry unresolved_sonames carries consuming binary and soname | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_unresolved_warning_tracks_aggregate_count` |
| `ANL-038` | sonames_consumed records binary, provider, soname per resolved link | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_linker_consumed_provided_shapes_and_dedup` |
| `ANL-039` | sonames_provided dedups sonames credited to a provider | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_linker_consumed_provided_shapes_and_dedup` |
| `ANL-040` | Linker walk reaches fixpoint following new providers transitively | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_linker_consumed_provided_shapes_and_dedup` |
| `ANL-041` | ELF reader follows DT_RUNPATH over DT_RPATH when both present | [analyze_elf.rs](analyze_elf.rs)::`elf_reader_prefers_runpath_over_rpath_when_both_present`, [analyze_elf.rs](analyze_elf.rs)::`elf_reader_falls_back_to_rpath_when_no_runpath` |
| `ANL-042` | ELF reader collects DT_NEEDED sonames in declaration order | [analyze_elf.rs](analyze_elf.rs)::`elf_reader_collects_needed_in_declaration_order` |
| `ANL-043` | ELF reader returns the SONAME a shared library advertises | [analyze_elf.rs](analyze_elf.rs)::`elf_reader_returns_soname_when_declared_and_none_when_absent` |
| `ANL-045` | ELF reader errors on unknown class byte and corrupt string offset | [analyze_elf.rs](analyze_elf.rs)::`elf_reader_errors_on_unknown_class_byte`, [analyze_elf.rs](analyze_elf.rs)::`elf_reader_errors_on_string_offset_out_of_range` |
| `ANL-046` | ELF reader treats binary with no dynamic section as no deps | [analyze_elf.rs](analyze_elf.rs)::`elf_reader_treats_no_dynamic_section_as_no_deps`, [analyze_elf.rs](analyze_elf.rs)::`elf_reader_returns_none_for_non_elf_bytes` |
| `ANL-047` | ElfScan includes bin dirs and .so files, prunes metadata/pseudo-fs/docs | [analyze_elf.rs](analyze_elf.rs)::`elf_scan_selects_bin_and_so_prunes_metadata_and_skips_symlinks` |
| `ANL-048` | soname_provider resolution uses ELF class and runpath per binary | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_linker_skips_declared_pass` |
| `ANL-049` | Analyze takes only the first --arch entry when several given | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_takes_only_first_arch_when_several_given` |
| `ANL-050` | Analyze on a different distro/format axis works end-to-end | [analyze_fedora.rs](analyze_fedora.rs)::`analyze_fedora_all_formats_share_one_package_set` |
| `ANL-052` | --with recommends/suggests has no effect under --strategy linker | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_with_is_inert_under_linker_strategy` |
| `ANL-053` | --with recommends widens declared closure (Debian Recommends) | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_with_widens_declared_closure_via_flag_and_env` |
| `ANL-054` | Missing --from rejected before analysis | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_missing_from_rejected_before_analysis` |
| `ANL-055` | Invalid --arch token rejected before analysis | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_invalid_arch_rejected_before_analysis` |
| `ANL-057` | Invalid --type/--format/--strategy/--with value rejected by clap | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_invalid_format_value_rejected_by_clap` |
| `ANL-058` | Provider missing from index after lookup is a hard error | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_linker_internal_invariants_do_not_fire_on_normal_input` |
| `ANL-059` | Downloader fetch failing for a frontier package errors the walk | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_linker_internal_invariants_do_not_fire_on_normal_input` |
| `ANL-060` | --http-retries env fallback feeds the linker download budget | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_http_retries_env_feeds_linker_download_budget` |
| `ANL-061` | Linker walk reuses the per-source archive cache across runs | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_linker_reuses_archive_cache_across_runs` |
| `ANL-062` | ArchContext open prints loaded-package count framing | [analyze_strategy.rs](analyze_strategy.rs)::`analyze_default_plain_frames_single_seed_and_marks_target`, [analyze_strategy.rs](analyze_strategy.rs)::`analyze_strategy_linker_skips_declared_pass` |
| `ANL-065` | trace `--type path --match any` unions seeds (explicit) | [../src/commands/analyze/trace.rs](../src/commands/analyze/trace.rs)::`targets_resolve_path_union_explicit` |
| `ANL-066` | trace `--match all` empty pattern is silent empty (contrast install) | [../src/commands/analyze/trace.rs](../src/commands/analyze/trace.rs)::`targets_resolve_intersect_empty_pattern_is_silent_empty` |
| `ANL-067` | trace `--match all` single pattern is identity | [../src/commands/analyze/trace.rs](../src/commands/analyze/trace.rs)::`targets_resolve_intersect_single_pattern_identity` |
| `ANL-069` | trace `--type path --match all` seeds only the common owner (live) | [cli_network.rs](cli_network.rs)::`trace_path_match_all_seeds_common_owner` |
| `ANL-070` | trace `--match all` disjoint → empty outcome, exit 0, zero stdout (live) | [cli_network.rs](cli_network.rs)::`trace_path_match_all_disjoint_empty_outcome` |
| `ANL-071` | trace `--type path --match all` seeds the common owner | [../src/commands/analyze/trace.rs](../src/commands/analyze/trace.rs)::`targets_resolve_path_intersect_picks_the_common_owner` |
| `ANL-072` | trace `--match all` disjoint is empty, not an error | [../src/commands/analyze/trace.rs](../src/commands/analyze/trace.rs)::`targets_resolve_path_intersect_disjoint_is_empty_not_error` |
| `ANL-073` | trace `--type library --match all` seeds the common owner | [../src/commands/analyze/trace.rs](../src/commands/analyze/trace.rs)::`targets_resolve_library_intersect_picks_common_owner` |
| `ANL-074` | trace `--type path --match all` on apk refused (live) | [cli_network.rs](cli_network.rs)::`trace_path_match_all_on_apk_refused` |

## Core: arch, version, manifest, download/cache, plain/json (47)

| ID | Scenario | Covering test |
|---|---|---|
| `CORE-005` | Guardrail refuses same distro but different release | [manifest_integration.rs](manifest_integration.rs)::`same_distro_different_release_is_refused_unchanged` |
| `CORE-010` | merge() replaces a record on changed version/checksum and logs it | [manifest_integration.rs](manifest_integration.rs)::`merge_replaces_a_record_when_version_or_checksum_differs` |
| `CORE-011` | merge() recomputes Sources and Architectures from final package set | [manifest_integration.rs](manifest_integration.rs)::`merge_recomputes_sources_and_architectures_from_the_final_set` |
| `CORE-012` | write() prunes stale files/<key> ledgers no longer referenced | [manifest_integration.rs](manifest_integration.rs)::`write_prunes_stale_ledgers_and_leftover_tmp_files` |
| `CORE-013` | Read rejects a manifest whose PackageCount disagrees with record count | [manifest_integration.rs](manifest_integration.rs)::`read_rejects_packagecount_disagreeing_with_record_count` |
| `CORE-014` | Read rejects when Sources/Architectures header disagrees with records | [manifest_integration.rs](manifest_integration.rs)::`read_rejects_sources_or_architectures_header_disagreeing_with_records` |
| `CORE-015` | Read errors when a package record has no matching files/ ledger | [manifest_integration.rs](manifest_integration.rs)::`read_errors_when_a_record_has_no_matching_files_ledger` |
| `CORE-016` | Read errors on missing required manifest keys / unknown keys | [manifest_integration.rs](manifest_integration.rs)::`read_rejects_missing_keys_unknown_header_key_and_non_integer_count` |
| `CORE-017` | Read errors when package data exists but manifest/packages missing | [manifest_integration.rs](manifest_integration.rs)::`read_errors_on_package_data_without_a_manifest_file_and_inverse` |
| `CORE-018` | PackageRecord stanza parse rejects unknown field keys | [manifest_integration.rs](manifest_integration.rs)::`read_rejects_unknown_record_field_and_missing_required_record_field` |
| `CORE-019` | files_format normalizes/sorts/dedups absolute; files_parse strips slash | [manifest_integration.rs](manifest_integration.rs)::`files_ledger_is_normalized_sorted_deduped_absolute_and_drops_bare_root` |
| `CORE-020` | Depends: field round-trips alternatives, constraints, empty list | [manifest_integration.rs](manifest_integration.rs)::`depends_field_round_trips_alternatives_constraints_and_empty_without_pipe_leak` |
| `CORE-021` | is_metadata excludes only .flatroot subtree from extraction sweep | [manifest_integration.rs](manifest_integration.rs)::`install_extraction_sweep_skips_only_the_flatroot_subtree` |
| `CORE-022` | Checksum format/parse round-trips all three algorithms; unknown refused | [manifest_integration.rs](manifest_integration.rs)::`checksum_format_parse_round_trips_all_algorithms_and_rejects_bad_input` |
| `CORE-023` | ChecksumAlgorithm::infer dispatches on digest shape | [manifest_integration.rs](manifest_integration.rs)::`checksum_algorithm_infer_dispatches_on_digest_shape` |
| `CORE-024` | Checksum::verify accepts genuine SHA-256/512 archives, rejects corruption | [download_cache.rs](download_cache.rs)::`checksum_verify_accepts_byte_exact_and_rejects_a_flipped_byte` |
| `CORE-025` | Checksum::verify for Alpine Q1-SHA1 hashes second gzip member only | [download_cache.rs](download_cache.rs)::`checksum_verify_q1_sha1_hashes_only_the_second_gzip_member` |
| `CORE-026` | Checksum-verified cache reuse without re-download | [download_cache.rs](download_cache.rs)::`checksum_infer_dispatches_on_digest_shape_and_drives_reuse`, [download_cache.rs](download_cache.rs)::`install_reuses_verified_cache_across_separate_runs` |
| `CORE-027` | Downloader re-downloads on mismatch, retries once, then hard-fails | [download_cache.rs](download_cache.rs)::`install_re_downloads_a_checksum_mismatched_cache_entry` |
| `CORE-028` | Cache reuse across separate runs persists in resolved cache home | [download_cache.rs](download_cache.rs)::`install_reuses_verified_cache_across_separate_runs` |
| `CORE-029` | FLATROOT_CACHE_HOME override; XDG and HOME fallbacks | [download_cache.rs](download_cache.rs)::`cache_dir_resolve_honors_precedence_chain_and_creates_the_dir` |
| `CORE-030` | Parallel download bound by -p / env (default 4) | [download_cache.rs](download_cache.rs)::`install_parallel_flag_is_honored_and_succeeds` |
| `CORE-031` | --http-retries drives stream_to and HttpClient budget (default 3) | [download_cache.rs](download_cache.rs)::`http_get_fresh_retries_then_hard_errors_after_the_budget` |
| `CORE-032` | Index listing cache honors TTL: fresh served local, stale refetched | [download_cache.rs](download_cache.rs)::`http_get_cached_honors_ttl_and_names_the_file_by_url_sha256` |
| `CORE-033` | get_fresh bypasses cache and streams unconditionally with progress | [download_cache.rs](download_cache.rs)::`http_get_fresh_always_hits_the_network_even_with_a_warm_cache` |
| `CORE-034` | HEAD probe returns presence cheaply: 2xx→true, 404→false, other→retry | [download_cache.rs](download_cache.rs)::`http_head_classifies_present_absent_and_retries_inconclusive` |
| `CORE-035` | Arch::from_uname accepts kernel vocab + both 32-bit spellings; rejects unknown | [arch_naming.rs](arch_naming.rs)::`as_uname_spells_every_target_and_round_trips`, [arch_naming.rs](arch_naming.rs)::`from_uname_accepts_both_32bit_spellings_and_rejects_unknown`, [download_cache.rs](download_cache.rs)::`install_unknown_arch_bails_before_any_work` |
| `CORE-036` | --arch default resolves to the host architecture | [arch_naming.rs](arch_naming.rs)::`from_host_resolves_to_a_supported_target` |
| `CORE-037` | Multilib --arch x86_64,i686 runs pipeline once per arch into one tree | [arch_qemu.rs](arch_qemu.rs)::`multilib_x86_64_i686_records_both_arches_in_one_manifest` |
| `CORE-038` | Per-distro arch naming renders the correct ecosystem spelling | [arch_naming.rs](arch_naming.rs)::`as_uname_spells_every_target_and_round_trips`, [arch_naming.rs](arch_naming.rs)::`as_goarch_spells_the_oci_platform_vocabulary`, [arch_naming.rs](arch_naming.rs)::`oci_variant_is_present_only_for_armv7l`, [arch_naming.rs](arch_naming.rs)::`as_debian_spells_the_deb_family_vocabulary`, [arch_naming.rs](arch_naming.rs)::`as_alpine_agrees_with_kernel_except_the_two_32bit_targets`, [arch_naming.rs](arch_naming.rs)::`as_gnu_triplet_names_the_multiarch_library_dir`, [arch_naming.rs](arch_naming.rs)::`each_renderer_is_injective_across_targets` |
| `CORE-039` | Arch::detect reads the rootfs's own ELF binaries (e_machine) | [arch_qemu.rs](arch_qemu.rs)::`oci_export_detects_and_stamps_the_rootfs_arch` |
| `CORE-040` | Arch::detect errors when no recognizable/readable ELF binary found | [arch_qemu.rs](arch_qemu.rs)::`export_errors_when_no_recognizable_elf_binary_is_found` |
| `CORE-041` | dpkg comparator honors epoch/upstream/revision and tilde | [version_compare.rs](version_compare.rs)::`dpkg_compare_honors_epoch_upstream_revision_and_tilde`, [version_compare.rs](version_compare.rs)::`dpkg_satisfies_honors_operators_against_constraints` |
| `CORE-042` | rpm comparator honors epoch/segments/tilde and single-char ops | [version_compare.rs](version_compare.rs)::`rpm_compare_honors_epoch_segments_and_tilde`, [version_compare.rs](version_compare.rs)::`rpm_satisfies_accepts_single_char_and_two_char_operators` |
| `CORE-043` | Version comparator lenient on malformed/empty constraints | [version_compare.rs](version_compare.rs)::`both_families_are_lenient_on_malformed_constraints` |
| `CORE-044` | Plain encoder emits one line per leaf with lex-sorted dotted keys | [plain_json.rs](plain_json.rs)::`plain_emits_one_line_per_leaf_with_lex_sorted_keys`, [plain_json.rs](plain_json.rs)::`plain_empty_container_emits_zero_bytes`, [plain_json.rs](plain_json.rs)::`remote_list_plain_is_a_lossless_mirror_of_json` |
| `CORE-045` | Plain encoder indexes arrays by position; preserves insertion order | [plain_json.rs](plain_json.rs)::`plain_indexes_arrays_by_position_in_insertion_order`, [plain_json.rs](plain_json.rs)::`remote_list_plain_is_a_lossless_mirror_of_json` |
| `CORE-047` | Plain encoder with_scope prefixes every line; scopes concatenate | [plain_json.rs](plain_json.rs)::`plain_scope_prefixes_every_line_and_chains_concatenate` |
| `CORE-048` | Plain encoder rejects keys/scopes/leaves breaking parseability | [plain_json.rs](plain_json.rs)::`plain_rejects_unparseable_keys_scopes_and_leaves_at_render`, [plain_json.rs](plain_json.rs)::`plain_first_error_poisons_further_pushes` |
| `CORE-049` | Plain encoder enforces single overall document shape | [plain_json.rs](plain_json.rs)::`plain_array_bails_on_a_non_empty_object_root`, [plain_json.rs](plain_json.rs)::`plain_pushing_object_content_into_an_array_root_poisons`, [plain_json.rs](plain_json.rs)::`plain_array_on_an_empty_printer_makes_the_root_an_array` |
| `CORE-050` | Format flags are bijective: json mirrors plain leaf-for-leaf | [plain_json.rs](plain_json.rs)::`remote_list_plain_is_a_lossless_mirror_of_json` |
| `CORE-051` | Write durability: atomic temp-then-rename per file; parent fsync best-effort | [download_cache.rs](download_cache.rs)::`fs_write_atomic_swaps_whole_and_keeps_prior_on_no_write` |
| `CORE-052` | FLATROOT_ARG_* env fallbacks supply flag values when flags absent | [download_cache.rs](download_cache.rs)::`install_without_output_bails_with_the_exact_message`, [download_cache.rs](download_cache.rs)::`install_env_fallbacks_supply_from_arch_and_output` |
| `CORE-053` | Empty existing sources set admits anything (defensive baseline) | [manifest_integration.rs](manifest_integration.rs)::`empty_existing_sources_set_admits_any_incoming_source` |
| `CORE-054` | files_load attributes every rootfs file; empty ledger is legitimate | [manifest_integration.rs](manifest_integration.rs)::`files_load_attributes_every_file_and_accepts_an_empty_ledger` |
| `CORE-062` | Manifest write durability ordering: files→packages→manifest→prune; .tmp swept | [manifest_integration.rs](manifest_integration.rs)::`manifest_write_orders_files_then_packages_then_manifest_then_prune` |
| `CORE-063` | Cross-distro guardrail axis: every ordered pair of 10 prefixes refused | [crosscutting_runtime.rs](crosscutting_runtime.rs)::`cross_distro_install_refused_across_ordered_pairs` |

## Output format (plain/json) — `--match` subset (3)

| ID | Scenario | Covering test |
|---|---|---|
| `FMT-001` | search `--match all --format json` carries only the common owner (live) | [cli_network.rs](cli_network.rs)::`search_path_match_all_json_carries_only_common_owner` |
| `FMT-002` | search `--match all` disjoint `--format json` is `[]` (live) | [cli_network.rs](cli_network.rs)::`search_path_match_all_disjoint_json_is_empty_array` |
| `FMT-003` | trace `--match all` disjoint `--format json` is `[]` (live) | [cli_network.rs](cli_network.rs)::`trace_path_match_all_disjoint_json_is_empty_array` |