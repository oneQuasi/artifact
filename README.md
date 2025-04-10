# `artifact`

## Overview

`artifact` is a fairy Chess engine which is currently in actively development. Currently, only standard chess is supported, but widespread compatibility is an active priority.

List of current features:
- Search:
    - [Negamax](https://www.chessprogramming.org/Negamax)
    - [Alpha-Beta Pruning](https://www.chessprogramming.org/Alpha-Beta)
    - [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search) (capture only)
    - [Iterative Deepening](https://www.chessprogramming.org/Iterative_Deepening)
    - [Transposition Table](https://www.chessprogramming.org/Transposition_Table) cutoffs and ordering
    - [Principle Variation Search](https://www.chessprogramming.org/Principal_Variation_Search)
    - [Late Move Reductions](https://www.chessprogramming.org/Late_Move_Reductions)
    - [Reverse Futility Pruning](https://www.chessprogramming.org/Reverse_Futility_Pruning)
    - [Null Move Pruning](https://www.chessprogramming.org/Null_Move_Pruning)
    - Move Ordering:
        - [MVV-LVA](https://www.chessprogramming.org/MVV-LVA)
        - [History Heuristic](https://www.chessprogramming.org/History_Heuristic)
- Evaluation:
    - [Stalemate](https://www.chessprogramming.org/Stalemate)
    - [Checkmate](https://www.chessprogramming.org/Checkmate), prioritizes faster checkmates
    - [Material](https://www.chessprogramming.org/Material)
    - [Piece-Square Tables](https://www.chessprogramming.org/Piece-Square_Tables)
    - [Tapered Eval](https://www.chessprogramming.org/Tapered_Eval)