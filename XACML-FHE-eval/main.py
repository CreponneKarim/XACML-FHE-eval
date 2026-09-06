import pandas as pd
import numpy as np
import matplotlib
pgf = False
pgf_font_size = 36
legend_font_size = 36
if pgf : 
	matplotlib.use("pgf")
	matplotlib.rcParams.update({
		"pgf.texsystem": "pdflatex",
		'font.family': 'serif',
		'font.size' : pgf_font_size,
	    "text.usetex": True,
	    "pgf.rcfonts": False,
	})
else:
	matplotlib.rcParams.update(matplotlib.rcParamsDefault)

	matplotlib.rcParams.update({
		"font.family": "serif",
		"font.serif": ["Times New Roman", "Times", "DejaVu Serif"],
		"font.size": pgf_font_size,
		"text.usetex": False,  
	})

import matplotlib.pyplot as plt
from matplotlib.patches import Patch

if not pgf:
	plt.rcParams['font.size'] = pgf_font_size

from scipy.interpolate import interp1d
# resets
# matplotlib.rcParams.update(matplotlib.rcParamsDefault)

data_path = "metrics_ops.csv"

df = pd.read_csv(data_path)
df = df.drop(columns=["options","total_us","stddev_us","rse_percent","iterations"])
df = df[df["name"] != "if-else"]


col_name = "Operation Category"
df[col_name] = "arithemetic"
col = df.pop(col_name)
df.insert(0, col_name, col)
df["avg_us"] = df["avg_us"] /1000
df["avg_us"] = df["avg_us"].map(lambda x: f"{x:.2f}")
df = df.rename(columns={"avg_us":"time"})


df["time"] = df["time"].astype(float)
df["name"] = df["name"].str.strip()
# pd.set_option("styler.latex.hrules", True)
latex_table = df.to_latex(
    index=True,
	longtable=True,
    column_format="|c|l|c|c|",
    caption="XACML TFHE operations evaluation using the tfhe-rs library",
    label="tab:ops_evaluation",
    escape=True
)
latex_table = latex_table.replace("\\toprule", "\\hline")
latex_table = latex_table.replace("\\midrule", "\\hline")
latex_table = latex_table.replace("\\bottomrule", "\\hline")

with open("table.tex", "w") as f:
    f.write(latex_table)


unique_values = df["name"].unique()
len(unique_values)

#	all string functions add either 5,10,15,20 chars to the name and 
avg_time_col_name = "time"
suffixes = [5, 10, 15, 20]

df = df.copy()
df["_orig"] = range(len(df))

# sort repeated names by value ascending
df = df.sort_values(["name", avg_time_col_name], ascending=[True, True])
tmp = df.copy()

# [tmp["name"].isin(["string-equal_5","string-equal_10","string-equal_15","string-equal_20"]) ]
# position inside each name group
order = tmp.groupby("name").cumcount()

# group size for each row
sizes = tmp.groupby("name")["name"].transform("size")

# build new names:
# - if size == 1: keep name as-is
# - if size > 1: append 5,10,15,20 according to sorted order
tmp["name"] = tmp["name"].where(
    sizes == 1,
    tmp["name"] + "-(" + order.map(lambda i: str(suffixes[i]) + " chars)")
)

# restore original row order if you want
df = tmp.sort_values("_orig").drop(columns="_orig")
# df[df["name"].isin(["string-equal_5","string-equal_10","string-equal_15","string-equal_20"]) ]

# create classes here
ascending=True
sortby = "time" 

#	arithmetic
mask_arithmetic = df["name"].str.contains("add|subtract|multiply|divide|abs",case=False)
arithmetic = df[mask_arithmetic].sort_values(by=sortby, ascending=ascending)

#	comparison
mask_comparison = df["name"].str.contains(r"equal(?!\))",regex=True,case=False) | df["name"].str.contains("less-than-or-equal|greater-than-or-equal|less-than|greater-than|equal-ignore-case",case=False)
comparison = df[mask_comparison].sort_values(by=sortby, ascending=ascending)

#	logic
mask_logic = df["name"].isin(["and","or","not","if-else"])
logic = df[mask_logic].sort_values(by=sortby, ascending=ascending)

#	bag
mask_bag = df["name"].str.contains("is-in")
bag = df[mask_bag].sort_values(by=sortby, ascending=ascending)

#	Uncategorized
mask_uncategorized = ~(mask_arithmetic | mask_comparison | mask_logic | mask_bag)
uncat = df[mask_uncategorized].sort_values(by=sortby, ascending=ascending)



def plot_plot(
	ax,
	div_set,
	colors,
	letter,
	ax_margin
):
	ax.text(
        0.02, 0.98, f"({letter})",
        transform=ax.transAxes,
        fontsize=legend_font_size,
        va="top",
        ha="left",
		rotation=90
    )

	for (d,c) in zip(div_set,colors):
		lbls = [
			f"{name} " + 
			# r"$\mathit{[" + 
			f"[{int(time)} ms]" 
			# + r"]}$"
			.strip()
			for name, time in zip(d["name"].astype(str), d["time"])
		]
		ax.bar(
			lbls,
			d["time"],
			# edgecolor="black",
			color= c
		)

		for label in ax.get_xticklabels():
			label.set_fontsize(pgf_font_size)
	
	
	ax.grid(True)
	ax.grid(color="gray")
	ax.set_axisbelow(True)
	ax.margins(x=ax_margin)

	ax.tick_params(axis="x", labelsize=pgf_font_size, rotation=90)
	ax.tick_params(axis="y", labelsize=pgf_font_size, rotation=90)

	# Thicker plot border
	for spine in ax.spines.values():
		spine.set_linewidth(2.0)

	for label in ax.get_xticklabels():
		label.set_fontsize(pgf_font_size)


# divide the dataset using the most consuming functions to the least consuming functions
division_lines1 = [-0.001,150.0,300.0, 600.0]
division_lines2 = [150.0,300.0,600.0,1000000.0]

divs = []
ds = [comparison, arithmetic, logic, bag, uncat]

for (d1,d2) in zip(division_lines1,division_lines2):
	divs.append(
		[d[(d["time"]<=d2) & (d["time"]>d1)] for d in ds]
	)	
# for dataset in :
# colors =["skyblue", "gold", "salmon", "lightgreen"]
colors =["skyblue", "gold", "salmon", "firebrick", "lightgreen"]
labels = ["Comparison", "Arithmetic", "Logic", "Bag", "Uncategorized"]

legend_handles = [
    Patch(facecolor=c, edgecolor="black", label=l)
    for c,l in zip(colors, labels)
]

widths = [sum([d.shape[0] for d in inner_div_set]) for inner_div_set in divs]
width = sum(widths)/2
height = width/2 + width/3

plt.clf()

# w_ratio = 3.3 * widths[1] /5
w_ratio = 0
fig, axes = plt.subplot_mosaic(
    [
        ["big", "big", "big","big","big"],
		[".",   ".",   ".", ".", "."], 
		[".",   ".",   ".", ".", "."], 
		[".",   ".",   ".", ".", "."], 
        ["small1", ".", "small2", ".","small3"],
    ],
	figsize=(width,height),
	gridspec_kw={
        "height_ratios": [3,1,1.6,1,3],
        "width_ratios": [ widths[1],w_ratio,  widths[2],w_ratio, widths[3]],  # small2 is twice as wide
    }
)

fig.subplots_adjust(wspace=0.21) 

	
fig.text(0.09, .75, "time (ms)", va="center", rotation="vertical",fontsize=pgf_font_size)


fig.legend(
    handles=legend_handles,
    loc="upper center",
    ncol=len(legend_handles),
    bbox_to_anchor=(0.5, 0.93)
)


# naming graphs with letters
letters = ['a','b','c','d','e']
last_index = len(divs)-1

ax_margin = .1
plot_plot(axes["big"],divs[0],colors,'class A',0)
plot_plot(axes["small1"],divs[1],colors,'class B',ax_margin)
plot_plot(axes["small2"],divs[2],colors,'class C',ax_margin)
plot_plot(axes["small3"],divs[3],colors,'class D',ax_margin)


if pgf:
	p = f"result.pgf"
	plt.savefig(p,bbox_inches="tight", pad_inches=0)
else:
	p = f"result.png"
	plt.savefig(p,bbox_inches="tight", pad_inches=0)


