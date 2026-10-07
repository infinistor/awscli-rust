use utf8;
use strict;
# CommandDispatcher.cs의 case를 (메뉴, Rust로 바꾼 도움말 식, 나머지 C# 본문)으로 출력한다.
# 사용: perl tools/dispatcher-cases.pl <시작 줄> <끝 줄>   (TESTCore 경로는 TESTCORE_SRC, 기본 E:/Code/Git/TESTCore)
my ($from, $to) = @ARGV;
my $src = $ENV{TESTCORE_SRC} // "E:/Code/Git/TESTCore";
open my $f, "<:utf8", "$src/Commands/CommandDispatcher.cs" or die "$src: $!";
my @lines = <$f>;
binmode STDOUT, ":utf8";
my $text = join "", @lines[$from - 1 .. $to - 1];

sub conv_args {
    my ($fn, $args) = @_;
    my @a = split_args($args);
    my ($value, @rest) = @a;
    my %named;
    my @pos;
    for (@rest) {
        if (/^(\w+):\s*(.*)$/s) { $named{$1} = $2 } else { push @pos, $_ }
    }
    my $v = conv_expr($value);
    if ($fn eq "MainFlag") {
        my $s = $pos[0] // $named{summary} // '""';
        return "usage::main_flag($v, " . conv_expr($s) . ")";
    }
    if ($fn eq "Optional") {
        my $s = $pos[0] // $named{summary} // '""';
        return "usage::optional($v, " . conv_expr($s) . ")";
    }
    # SubFlag, OptionalValue, MainFlagValue: (value, type = "string", summary = "")
    my $type = $pos[0] // $named{type} // '"string"';
    my $summary = $pos[1] // $named{summary} // '""';
    my %rust = (SubFlag => "sub_flag", OptionalValue => "optional_value", MainFlagValue => "main_flag_value");
    return "usage::$rust{$fn}($v, " . conv_expr($type) . ", " . conv_expr($summary) . ")";
}

sub split_args {
    my $s = shift;
    my @out;
    my $depth = 0;
    my $cur = "";
    my $in_str = 0;
    for my $c (split //, $s) {
        if ($in_str) { $cur .= $c; $in_str = 0 if $c eq '"'; next }
        if ($c eq '"') { $in_str = 1; $cur .= $c; next }
        $depth++ if $c eq '(';
        $depth-- if $c eq ')';
        if ($c eq ',' && $depth == 0) { push @out, trim($cur); $cur = ""; next }
        $cur .= $c;
    }
    push @out, trim($cur) if trim($cur) ne "";
    return @out;
}

sub trim { my $s = shift; $s =~ s/^\s+|\s+$//g; return $s }

sub conv_expr {
    my $e = trim(shift);
    if ($e =~ /^Usage\.(MainFlag|SubFlag|Optional|OptionalValue|MainFlagValue)\((.*)\)$/s) { return conv_args($1, $2) }
    if ($e =~ /^Usage\.(\w+)$/) { return "usage::$1" }
    if ($e =~ /^"/) { return $e }
    return "/* ?? $e */";
}

sub conv_help {
    my $expr = shift;
    my @parts = split_plus($expr);
    my @r = map { conv_expr($_) } @parts;
    return "[" . join(",\n    ", map { $_ =~ /^usage::[A-Z_0-9]+$|^"/ ? "$_.to_string()" : $_ } @r) . "].concat()" if @r > 1;
    return $r[0];
}

sub split_plus {
    my $s = shift;
    my @out;
    my $depth = 0;
    my $cur = "";
    my $in_str = 0;
    for my $c (split //, $s) {
        if ($in_str) { $cur .= $c; $in_str = 0 if $c eq '"'; next }
        if ($c eq '"') { $in_str = 1; $cur .= $c; next }
        $depth++ if $c eq '(';
        $depth-- if $c eq ')';
        if ($c eq '+' && $depth == 0) { push @out, trim($cur); $cur = ""; next }
        $cur .= $c;
    }
    push @out, trim($cur);
    return @out;
}

while ($text =~ /case MenuList\.(\w+):\s*\{(.*?)\n\t\t\t\t\t\tbreak;\s*\n\t\t\t\t\t\}/sg) {
    my ($menu, $body) = ($1, $2);
    print "=== $menu\n";
    if ($body =~ /if \(help\)\s*(?:\{\s*)?Console\.WriteLine\((.*?)\);\s*(?:\})?\s*\n\s*else/s) {
        print "HELP: ", conv_help($1), "\n";
    } else {
        print "HELP: <manual>\n";
    }
    (my $rest = $body) =~ s/^.*?\n\s*else\b//s;
    $rest =~ s/^\s+//;
    $rest =~ s/\n\t{6}/\n/g;
    print "REST: $rest\n";
}
