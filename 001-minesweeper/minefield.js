"use strict";

class MinefieldCell {
    constructor($elm, i, j, mined) {
        this.$elm = $elm
        this.i = i
        this.j = j
        this.mined = mined
        this.flagged = false
        this.surroundingMines = 0
        this.surroundingFlags = 0
        this.surroundingUncovered = 0
        this.uncovered = false
        this.numberOfPeers = 0
    }

    updateUI() {
        if (this.flagged) {
            this.$elm.className = 'flagged'
            this.$elm.innerText = 'F'
        } else if (!this.uncovered) {
            this.$elm.className = 'covered'
            this.$elm.innerText = ''
        } else if (this.mined) {
            this.$elm.className = 'mined'
            this.$elm.innerText = 'X'
        } else if (this.surroundingMines > 0) {
            this.$elm.className = ''
            this.$elm.innerText = this.surroundingMines.toString()
        } else {
            this.$elm.className = ''
            this.$elm.innerText = ''
        }
    }
}

class Minefield {
    constructor($table, rows, cols, minerate) {
        this.$table = $table
        this.rows = rows
        this.cols = cols
        this.grid = []

        for (let i = 0; i < this.rows; i++) {
            let $row = document.createElement('div'),
                row = []
            for (let j = 0; j < this.cols; j++) {
                let $elm = document.createElement('div')
                $elm.dataset.i = i
                $elm.dataset.j = j
                let cell = new MinefieldCell($elm, i, j, Math.random() < minerate)
                row.push(cell)
                $row.appendChild($elm)
            }

            this.grid.push(row)
            $row.className = 'row'
            this.$table.appendChild($row)
        }

        for (let i = 0; i < this.rows; i++) {
            for (let j = 0; j < this.cols; j++) {
                let cell = this.grid[i][j]
                let numberOfPeers = 0
                cell.surroundingMines = this.iterPeers(cell, (peer, n) => {
                    numberOfPeers += 1
                    return peer.mined ? n + 1 : n
                }, 0)
                cell.numberOfPeers = numberOfPeers
                cell.updateUI()
            }
        }
    }

    iterPeers(cell, f, x0 = null) {
        let x = x0
        for (let di = -1; di < 2; di++) {
            for (let dj = -1; dj < 2; dj++) {
                if (di == 0 && dj == 0)
                    continue

                let i = cell.i + di,
                    j = cell.j + dj
                if (i < 0 || j < 0 || i >= this.rows || j >= this.cols)
                    continue

                x = f(this.grid[i][j], x)
            }
        }
        return x
    }

    show() {
        for (let i = 0; i < this.rows; i++) {
            for (let j = 0; j < this.cols; j++) {
                let cell = this.grid[i][j]
                cell.uncovered = true
                cell.updateUI()
            }
        }
    }

    uncoverEmptyFieldsAround(cell0) {
        let uncover = (cell) => {
            if (cell.uncovered)
                return

            cell.uncovered = true
            this.iterPeers(cell, peer => peer.surroundingUncovered += 1)

            if (cell.surroundingMines == 0)
                this.iterPeers(cell, uncover)
            cell.updateUI()
        }
        this.iterPeers(cell0, uncover)
    }

    initGame() {
        let i = 0, j = 0, cell0 = null
        while (true) {
            i = Math.floor(Math.random() * this.rows)
            j = Math.floor(Math.random() * this.cols)

            cell0 = this.grid[i][j]
            if (!cell0.mined && cell0.surroundingMines == 0)
                break
        }

        this.uncoverEmptyFieldsAround(cell0)
    }

    uncover(cell) {
        if (cell.uncovered) {
            console.warn('Already uncovered...!')
            return
        }

        cell.uncovered = true
        this.iterPeers(cell, peer => peer.surroundingUncovered += 1)

        if (cell.surroundingMines == 0)
            this.uncoverEmptyFieldsAround(cell)

        cell.updateUI()
    }

    flag(cell) {
        cell.flagged = true
        this.iterPeers(cell, peer => {
            peer.surroundingFlags += 1
        })

        cell.updateUI()
    }
}
