use clap::Subcommand;

pub mod joox;
pub mod kgm;
pub mod mg3d;
pub mod ncm;
pub mod qmc1;
pub mod qmc2;
pub mod qtfm;
pub mod xiami;
pub mod xmly;

#[derive(Subcommand)]
pub enum Commands {
    #[command(name = "ncm")]
    Ncm(ncm::ArgsNCM),

    #[command(name = "kgm")]
    Kgm(kgm::ArgsKGM),

    #[command(name = "mg3d")]
    Migu3D(mg3d::ArgsMigu3D),

    #[command(name = "joox")]
    Joox(joox::ArgsJoox),

    #[command(name = "qmc1")]
    QMCv1(qmc1::ArgsQMCv1),
    #[command(name = "qmc2")]
    QMCv2(qmc2::ArgsQMCv2),

    #[command(name = "qtfm")]
    QtFM(qtfm::ArgsQingTingFM),

    #[command(name = "xiami")]
    Xiami(xiami::ArgsXiami),

    #[command(name = "xmly")]
    Xmly(xmly::ArgsXimalaya),
}
