; Simple indentation heuristics; refine with real grammar nodes later.

(("{" @indent.begin) ("}" @indent.end))
(("[" @indent.begin) ("]" @indent.end))
(("(" @indent.begin) (")" @indent.end))

