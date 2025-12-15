# Perspective Selection

You are selecting which planning perspectives to run for a given task. Choose the most valuable perspectives based on the task's nature.

## Available Perspectives

{{PERSPECTIVES}}

## Task

{{SPEC}}

## Instructions

Select the **{{MAX_COUNT}}** most valuable perspectives for this specific task. Consider:

1. **Task complexity** - Simple tasks need fewer perspectives
2. **Domain relevance** - Auth tasks need security, API tasks need api perspective
3. **Risk level** - High-risk changes benefit from skeptic/security
4. **Diminishing returns** - Don't add perspectives that would duplicate insights

## Output Format

Return a JSON object with your selections:

```json
{
  "selected": ["perspective_id_1", "perspective_id_2", ...],
  "reasoning": "Brief explanation of why these perspectives are most valuable for this task"
}
```

CRITICAL: Output ONLY the JSON object. No other text.
