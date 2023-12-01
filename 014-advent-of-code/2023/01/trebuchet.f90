program trebuchet
    implicit none
    character(100) :: line
    integer :: ioerr
    integer :: sum
    sum = 0
    do
        read(*, *, IOSTAT=ioerr) line
        if (ioerr > 0) then
            write(*, *) "Something went wrong...!"
            exit
        else if (ioerr < 0) then
            write(*, *) "The calibration number sum is: ", sum
            exit
        end if
        sum = sum + handle_line(line)
    end do
contains
    function handle_line(line) result(num)
        character, dimension(100), intent(in) :: line
        integer :: num

        integer :: i, ic, digits_start, digits_end
        character :: c, first_char, last_char
        character(len=3) :: digits

        first_char = ' '
        last_char = ' '
        digits_start = IACHAR('0')
        digits_end = IACHAR('9')

        do i = 1, 100
            c = line(i)
            ic = IACHAR(c)
            if (ic < digits_start) then
                cycle
            end if
            if (digits_end < ic) then
                cycle
            end if

            if (first_char == ' ') then
                first_char = c
            end if
            last_char = c
        end do

        digits = first_char // last_char // ' '
        read(digits, *) num
    end function handle_line
end program trebuchet
