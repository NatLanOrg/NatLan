# Scoring

How [checks](./checks.md#checks) combine into a final score and a verdict.

## Aggregation

- **Case score.** A weighted blend of the case's check sub-scores. The
  [schema check](./checks.md#schema-check) is a gate: if it fails, the case scores 0. Otherwise the
  score is mostly the [assertion](./checks.md#assertion-checks) sub-score, with the
  [judge](./checks.md#judge-check) sub-score contributing a smaller weight, since self-judging is a
  soft signal.
- **Stage score.** The mean of the scores of that stage's cases.
- **Overall score.** A weighted mean across stages. Stages graded mostly by schema and assertions
  count for more than stages that lean on the judge, so the overall number tracks the checks that are
  trustworthy.

## Throughput

Quality is not the only axis. A model that returns perfect artifacts but is slow to respond is
impractical to compile with, because the stages run thousands of times over a real project. The
benchmark measures generation speed while the cases run and reports it.

Speed has two independent parts, and a single tokens-per-second number hides the difference:

- **Time to first token (TTFT).** The wait from sending the request to the first output token. It is
  dominated by queueing and, for reasoning models, by the hidden thinking phase before any visible
  token. A model can decode fast yet have a multi-second TTFT on every call.
- **Output speed.** The token decode rate once generation has started, in tokens per second. This is
  the steady-state rate, independent of the startup wait.

To separate them the benchmark **streams** the response (`stream: true`) rather than awaiting the
whole body. It timestamps the first token (giving TTFT) and the first-to-last-token window (giving
output speed). Awaiting the whole response cannot tell the two apart: it only yields one blended rate
that, on the small outputs the stages produce, is dragged down by TTFT.

- **Measurement.** Both metrics are measured across all cases, not per case. Every streamed call
  contributes its TTFT, its decode window, and its output tokens. The benchmark accumulates them and
  reports, at the end, the mean TTFT and the aggregate output speed
  (`total output tokens / total decode seconds`). Token counts come from the endpoint's
  `usage.completion_tokens` when reported (requested via `stream_options.include_usage`), and are
  counted from the streamed deltas otherwise. The blended end-to-end rate
  (`tokens / (TTFT + decode)`) is also reported, for reference.
- **Output-speed score.** The output speed is compared against a reference target, `speedTarget`
  tokens per second. The sub-score is `min(1, output speed / speedTarget)`, so a model at or above
  target scores 1 and a slower model scores in proportion.
- **TTFT score.** The mean TTFT is compared against a reference target, `ttftTarget` seconds. The
  sub-score is `min(1, ttftTarget / TTFT)`, so a model at or under target scores 1 and a slower start
  scores in proportion. TTFT is a reported, soft signal: a reasoning model trades a high TTFT for
  better output, so it does not gate the verdict on its own.

The targets are reference values, not hardware-specific numbers, and are reported next to the measured
figures.

## Verdict

The benchmark answers one question: can this model compile Jazyk? The verdict is **usable** only when
all hold:

- The overall score clears an overall threshold.
- Every stage clears a per-stage floor.
- The output speed clears a minimum speed floor.

The per-stage floor matters because the stages are not interchangeable. A model that aces four stages
but cannot return conforming entities for A2 is unusable, even with a high average, because every later
stage depends on A2. A single failing stage caps the verdict at not-usable.

The output-speed floor matters for the same reason. The stages run thousands of times over a real
project, so a model that decodes below the floor turns a build into hours of waiting even when its
output is correct. An output speed below the floor caps the verdict at not-usable, regardless of the
quality scores. The floor sits well below the target: it rejects only models too slow to use, while the
target rewards models fast enough to be comfortable. TTFT is reported alongside but does not gate, since
a slow start with a fast decode is still workable over long generations.

## Output

Human-readable by default: per-stage scores, the overall score, the time to first token and output
speed with their scores, the verdict, and the cases that failed. A machine-readable JSON form is available for tooling
and for comparing models, following the same convention as the rest of the
[CLI](../cli.md#exit-codes).

The command exits `0` when the verdict is usable and non-zero otherwise, so it can gate CI or serve as
a pre-flight check before trusting a newly configured model or endpoint.
