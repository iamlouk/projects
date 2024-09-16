class MinesweeperSolver {
    constructor(field) {
        this.field = field
        this.failed = false
    }

    step(singlestep = true) {
        let changes = 0
        if (this.failed)
            return 0

        for (let i = 0; i < this.field.rows; i++) {
            for (let j = 0; j < this.field.cols; j++) {
                let cell = this.field.grid[i][j]
                const missingMines = cell.surroundingMines - cell.surroundingFlags
                const coveredPeers = cell.numberOfPeers - cell.surroundingUncovered - cell.surroundingFlags

                // Strategy 1: Place flags around us if our count says that they must contain some!
                if (!cell.flagged && cell.uncovered && missingMines > 0 && coveredPeers == missingMines) {
                    this.field.iterPeers(cell, c => {
                        if (!c.flagged && !c.uncovered) {
                            changes += 1
                            this.field.flag(c)

                            if (!c.mined) {
                                this.failed = true
                                console.warn('solver failed:', c)
                            }
                        }
                    })

                    if (singlestep)
                        return changes
                }

                // Strategy 2: Uncover cells if all mines in the are are found!
                if (!cell.flagged && cell.uncovered && missingMines == 0 && coveredPeers > 0) {
                    this.field.iterPeers(cell, c => {
                        if (!c.flagged && !c.uncovered) {
                            changes += 1
                            this.field.uncover(c)

                            if (c.mined) {
                                this.failed = true
                                console.warn('solver failed:', c)
                            }
                        }
                    })

                    if (singlestep)
                        return changes
                }
            }
        }

        console.log(`solver: #changes=${changes}`)
        return changes
    }
}
