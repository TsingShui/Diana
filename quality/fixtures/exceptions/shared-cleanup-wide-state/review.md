# Semantic review

Scheduling sees the original timestamp and original marker. Save commits the
middle marker only if it succeeds. Before reacquisition, a successful refresh
has changed the timestamp while the middle marker is still live. Refresh and
reacquisition failures therefore expose different pairs of values. A save
fault followed by a refresh fault preserves both initial values and exposes
the replacement throwable. A wait fault followed by successful exceptional
refresh and reacquisition exposes both newly committed values when rethrown.

Diana 0.4.5 loses the timestamp on nine modeled paths while preserving the
marker and throwable. Diana 0.4.6 saves the two values independently at each
cleanup entry. Its outer catch receives the expected pair on cancellation
failure and on rethrow. Normal completion calls the reporting hook with the
fresh pair after cancellation. The reporting hook's fault uses its own input
to the same catch and reaches the diagnostic without running cleanup again.
A successful typed interruption rejoins normal refresh.

The lexical `finally` contains one cancellation. The independent DEX path
witness and executable Java model establish the supplied call, value and
exception traces. They do not exercise framework code outside the fixture.
