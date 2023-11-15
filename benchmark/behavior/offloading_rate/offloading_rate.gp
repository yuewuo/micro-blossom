default_font = "Arial, 28"
set terminal postscript eps color default_font
set terminal postscript landscape enhanced
set xlabel "Code Distance" font default_font
set ylabel "#Offloaded / #Defects" font default_font
set title "Primal Offloading Rate at d {/Symbol \264} d {/Symbol \264} d Circuit-level Noise"
set size 1,1

set style line 12 lc rgb '0xCCCCCC' lt 1 lw 2
set grid ytics ls 12
set grid xtics ls 12

set lmargin 5
set rmargin 0
set tmargin 1
set bmargin 1

set logscale x
set xrange [2:24] 
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
# set xtics ('3' 3, '5' 5, '7' 7, '9' 10, '20' 20, '50' 50, '100' 100)
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set yrange [0:1]
# set style fill transparent solid 0.2 noborder
set key box top left

set output "offloading_rate.eps"

plot "data_p0.0005.txt" using 1:4 with linespoints lt rgb "orange" linewidth 3 pointtype 5 pointsize 1.5 title "p = 0.05\%",\
    "data_p0.001.txt" using 1:4 with linespoints lt rgb "red" linewidth 3 pointtype 5 pointsize 1.5 title "p = 0.1\%",\
    "data_p0.002.txt" using 1:4 with linespoints lt rgb "blue" linewidth 3 pointtype 5 pointsize 1.5 title "p = 0.2\%",\
    "data_p0.005.txt" using 1:4 with linespoints lt rgb "purple" linewidth 3 pointtype 5 pointsize 1.5 title "p = 0.5\%",\
    "data_p0.01.txt" using 1:4 with linespoints lt rgb "black" linewidth 3 pointtype 5 pointsize 1.5 title "p = 1\%"

system("ps2pdf -dEPSCrop offloading_rate.eps offloading_rate.pdf")

