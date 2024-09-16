
const SOLVER_SOLVED = 'solved',
      SOLVER_VIA_ELIMINATION = 'eliminations',
      SOLVER_SPECULATION_STARTED = 'speculation started...',
      SOLVER_SPECULATION_FAILED = 'speculation failed...',
      SOLVER_IMPOSSIBLE = 'sudoku impossible'

class Cell {
    constructor(sudoku, $elm, x, y, digit = 0, possibilities = null, fixed = false, marked = false) {
        this.digit = digit
        this.x = x
        this.y = y
        this.id = sudoku.size * x + y
        this.sudoku = sudoku
        this.fixed = fixed
        this.marked = marked

        if (possibilities != null) {
            this.possibilities = possibilities
        } else {
            this.possibilities = new Set()
            for (let d = 1; d <= sudoku.size; d++)
                this.possibilities.add(d)
        }

        this.$elm = $elm
    }

    updateUI() {
        this.$elm.innerText = this.digit == 0 ? '' : this.digit.toString()
        this.$elm.className = this.fixed ? 'fixed' : (this.marked ? 'marked' : '')
    }

    copy() {
        return new Cell(this.sudoku, this.$elm, this.x, this.y, this.digit, new Set(this.possibilities.values()), this.fixed, this.marked)
    }
}

class Sudoku {
    constructor($table, size = 9) {
        this.size = size
        this.grid = []
        this.$table = $table
        this.speculations = []


        this.digits = []
        for (let d = 1; d <= this.size; d++)
            this.digits.push(d)

        for (let x = 0; x < this.size; x++) {
            let row = []
            let $row = document.createElement('div')
            $row.className = 'row'

            for (let y = 0; y < this.size; y++) {
                let $cell = document.createElement('div')
                $cell.dataset.x = x
                $cell.dataset.y = y
                row.push(new Cell(this, $cell, x, y))
                $row.appendChild($cell)
            }

            this.grid.push(row)
            this.$table.appendChild($row)
        }

        this.units = []
        for (let x = 0; x < this.size; x++) {
            let row = [], col = []
            for (let y = 0; y < this.size; y++) {
                row.push({ x: x, y: y })
                col.push({ x: y, y: x })
            }

            this.units.push(row)
            this.units.push(col)
        }

        let subsquares = Math.floor(Math.sqrt(this.size))
        console.assert(subsquares * subsquares == this.size)
        for (let i = 0; i < subsquares; i++) {
            for (let j = 0; j < subsquares; j++) {
                let subsquare = []
                for (let di = 0; di < subsquares; di++) {
                    for (let dj = 0; dj < subsquares; dj++) {
                        let x = i * subsquares + di, y = j * subsquares + dj
                        subsquare.push({ x: x, y: y })
                    }
                }
                this.units.push(subsquare)
            }
        }

        this.cell2units = new Array(this.size*this.size).fill(0).map(_ => [])
        for (let unit of this.units) {
            for (let pos of unit) {
                let id = this.size * pos.x + pos.y
                this.cell2units[id].push(unit)
            }
        }
    }

    init(game) {
        console.assert(game.length == this.size)
        for (let i = 0; i < game.length; i++) {
            console.assert(game[i].length == this.size)
            for (let j = 0; j < game[i].length; j++) {
                let cell = this.grid[i][j]
                cell.digit = game[i][j]
                if (cell.digit != 0)
                    cell.fixed = true
            }
        }

        this.removePossibilities()
        this.updateUI()
    }

    removePossibilities() {
        let changed = false
        for (let unit of this.units) {
            for (let pos of unit) {
                let cell = this.grid[pos.x][pos.y]
                if (cell.digit == 0)
                    continue

                for (let peer of unit)
                    if (peer != pos)
                        changed |= this.grid[peer.x][peer.y].possibilities.delete(cell.digit)
            }
        }
        return changed
    }

    placeDigit(cell, digit) {
        cell.digit = digit
        let changed = false
        for (let unit of this.cell2units[cell.id]) {
            for (let peer of unit) {
                if (!(peer.x == cell.x && peer.y == cell.y)) {
                    changed ||= this.grid[peer.x][peer.y].possibilities.delete(digit)
                }
            }
        }

        return changed
    }

    placeDigits() {
        let changed = false
        for (let row of this.grid) {
            for (let cell of row) {
                if (cell.digit == 0 && cell.possibilities.size == 1) {
                    let digit = cell.possibilities.values().next().value
                    cell.digit = digit
                    changed = true
                    // this.placeDigit(cell, digit)
                } else if (cell.possibilities.size == 0) {
                    return { changed, failed: true }
                }
            }
        }

        return { changed, failed: false }
    }

    updateUI() {
        for (let row of this.grid) {
            for (let cell of row) {
                cell.updateUI()
            }
        }
    }

    check() {
        let solved = true
        for (let row of this.grid) {
            for (let cell of row) {
                if (cell.digit == 0)
                    solved = false
            }
        }

        let digits = new Array(this.size)
        for (let unit of this.units) {
            digits.fill(0)
            for (let pos of unit) {
                let cell = this.grid[pos.x][pos.y]
                if (cell.digit != 0) {
                    digits[cell.digit] += 1
                    if (digits[cell.digit] == 2) {
                        return { solved, failed: true }
                    }
                }
            }
        }

        return { solved, failed: false }
    }

    solve() {
        this.removePossibilities()
        let { changed, failed } = this.placeDigits()
        this.updateUI()
        if (changed && !failed)
            return SOLVER_VIA_ELIMINATION

        let status
        if (failed || (status = this.check()).failed) {
            let speculation = this.speculations.pop()
            if (speculation == null)
                return SOLVER_IMPOSSIBLE

            speculation.try += 1
            this.grid = speculation.originalGrid.map(row => row.map(cell => cell.copy()))
            let cell0 = this.grid[speculation.x][speculation.y]
            cell0.digit = speculation.possibilities[speculation.try]
            cell0.possibilities = new Set([cell0.digit])

            if (speculation.try + 1 < speculation.possibilities.length)
                this.speculations.push(speculation)

            this.updateUI()
            return SOLVER_SPECULATION_FAILED
        }

        if (status.solved)
            return SOLVER_SOLVED

        // Find cell with the lowest amount of possible values...
        let cell0 = null
        for (let row of this.grid) {
            for (let cell of row) {
                if (cell.digit == 0 && (cell0 == null || cell.possibilities.size < cell0.possibilities.size)) {
                    cell0 = cell
                }
            }
        }

        // Start a new speculation...
        let speculation = {
            x: cell0.x,
            y: cell0.y,
            possibilities: [...cell0.possibilities.values()],
            try: 0,
            originalGrid: this.grid.map(row => row.map(cell => cell.copy()))
        }
        this.speculations.push(speculation)
        cell0.digit = speculation.possibilities[0]
        cell0.possibilities = new Set([cell0.digit])
        cell0.marked = true
        cell0.updateUI()
        return SOLVER_SPECULATION_STARTED
    }
}